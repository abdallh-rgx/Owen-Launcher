/* ============================================================================
 * uefinder v6 — UE/Fortnite Android (ARM64 ELF) static function finder
 * Pure C99, single translation unit, zero external dependencies.
 *  - Decodes RELA / REL / Android Packed (APS2/APR2, LLD format) / RELR relocs
 *  - Function boundaries: .eh_frame_hdr table + full .eh_frame CIE/FDE walk
 *  - Path A: native FName  ->  {Name,Func} table  ->  exec thunk -> impl
 *  - Path B: log string    ->  ADRP+ADD code ref ->  containing function
 *    (UE5 outlined-log-thunk aware: BL-caller + tail-B parent resolution)
 *  - VTable index & base (slot runs in .data.rel.ro), vcall site analysis
 *  - Special modes: cksz (ConstructAbilitySpec), gis (GIsClient/GIsServer as
 *    VARIABLES), namepool (FNamePool / TNameEntryArray — MobileDumper-7 style
 *    anchors), vtbase (UNetConnection vtable base + method slots), chain,
 *    vftnext, anchor, encpatch, cvar, zc.
 *  - --all: enumerate every native UFunction -> offsets.h (curated first).
 * ==========================================================================*/
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>

#define VER "6.0"

/* ------------------------------------------------------------------ types */
typedef struct { uint16_t s; } E16_;
typedef struct {
    uint8_t  e_ident[16];
    uint16_t e_type, e_machine;
    uint32_t e_version;
    uint64_t e_entry, e_phoff, e_shoff;
    uint32_t e_flags;
    uint16_t e_ehsize, e_phentsize, e_phnum, e_shentsize, e_shnum, e_shstrndx;
} Elf64_Ehdr;
typedef struct {
    uint32_t p_type, p_flags;
    uint64_t p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align;
} Elf64_Phdr;
typedef struct {
    uint32_t sh_name, sh_type;
    uint64_t sh_flags, sh_addr, sh_offset, sh_size;
    uint32_t sh_link, sh_info;
    uint64_t sh_addralign, sh_entsize;
} Elf64_Shdr;
typedef struct {
    uint64_t d_tag;
    union { uint64_t d_val; uint64_t d_ptr; } d_un;
} Elf64_Dyn;
typedef struct {
    uint64_t r_offset;
    uint64_t r_info;
    int64_t  r_addend;
} Elf64_Rela;
typedef struct {
    uint64_t r_offset;
    uint64_t r_info;
} Elf64_Rel;

#define PT_LOAD_ 1
#define SHT_NOBITS_ 8
#define SHT_PROGBITS_ 1
#define SHF_WRITE_  0x1
#define SHF_ALLOC_  0x2
#define SHF_EXEC_   0x4

#define R_AARCH64_RELATIVE_ 1027

#define MAX_SECS 512

typedef struct {
    uint64_t off;   /* r_offset (VA) */
    uint64_t add;   /* addend (target VA) */
} RelSlot;

typedef struct { uint64_t tgt, pc; } RefSite;

typedef struct {
    uint64_t start, end;   /* [start, end) */
} FuncRange;

/* ------------------------------------------------------------- globals */
static uint8_t *D_; static uint64_t N_;          /* file image */
static Elf64_Ehdr *EH_;
static Elf64_Phdr *PH_; static int NPH_;
static Elf64_Shdr SH_[MAX_SECS]; static int NSH_;
static const char *SHSTR_;

static char SO_NAME[64];       /* ".text" etc. resolved indices */
static int I_TEXT = -1, I_RODATA = -1, I_DRELRO = -1, I_EHFRAME = -1,
           I_EHFRAMEHDR = -1, I_INITARRAY = -1;

static RelSlot *RS_; static uint64_t NRS_;       /* R_AARCH64_RELATIVE index sorted by off */
static uint64_t RS_CAP_;
static RefSite *REF_; static uint64_t NREF_;     /* ADRP+ADD refs sorted by tgt */
static RefSite *REFL_; static uint64_t NREFL_;   /* ADRP+LDR refs sorted by tgt */
static uint64_t *FS_; static uint64_t NFS_;      /* function starts sorted */
static uint64_t *FE_;                            /* function ends (extents)  */
static int       *FK_;                           /* FKIND per func index     */
static uint64_t *VT_SLOT_; static uint64_t VT_N_;/* vtable slot addrs sorted */
static uint64_t *VT_RUN_;  static uint64_t VT_NR_;/* run start for each slot  */
typedef struct { uint64_t pc, tgt; } CallSite;
static CallSite *BL_; static uint64_t NBL_;      /* BL sites sorted by tgt */

enum { FK_NORMAL = 0, FK_BRIDGE = 1, FK_LOGTHUNK = 2, FK_TINY = 3 };

/* ------------------------------------------------------------ small util */
static void *xmalloc(uint64_t n){ void*p=malloc(n?n:1); if(!p){fprintf(stderr,"OOM %llu\n",(unsigned long long)n); exit(2);} return p; }
static void *xrealloc(void*q,uint64_t n){ void*p=realloc(q,n?n:1); if(!p){fprintf(stderr,"OOM\n"); exit(2);} return p; }

static uint64_t rd32(const uint8_t*p){ return (uint64_t)p[0]|((uint64_t)p[1]<<8)|((uint64_t)p[2]<<16)|((uint64_t)p[3]<<24); }
static uint64_t rd64(const uint8_t*p){ return rd32(p)|((uint64_t)rd32(p+4)<<32); }

/* va -> file offset (through program headers). returns 0 when unmapped */
static uint64_t va2off(uint64_t va){
    for (int i=0;i<NPH_;i++)
        if (PH_[i].p_type==PT_LOAD_){
            uint64_t v=PH_[i].p_vaddr, f=PH_[i].p_filesz;
            if (va>=v && va<v+f) return PH_[i].p_offset + (va-v);
        }
    return 0;
}
static const uint8_t *va_ptr(uint64_t va, uint64_t len){
    uint64_t o = va2off(va);
    if (!o || o + len > N_) return NULL;
    return D_ + o;
}

/* section containing va (or NULL) */
static const Elf64_Shdr *sec_of(uint64_t va){
    for (int i=0;i<NSH_;i++){
        const Elf64_Shdr *s=&SH_[i];
        if (!s->sh_addr || !(s->sh_flags & SHF_ALLOC_)) continue;
        if (va>=s->sh_addr && va<s->sh_addr+s->sh_size) return s;
    }
    return NULL;
}
static const char *sec_name(const Elf64_Shdr *s){ return SHSTR_ + s->sh_name; }

static int sec_writable(const Elf64_Shdr *s){ return (s->sh_flags & SHF_WRITE_)!=0; }

/* is va inside .text range */
static uint64_t TEXT_VA_, TEXT_SZ_; static const uint8_t *TEXT_P_;

static int in_text(uint64_t va){ return TEXT_SZ_ && va>=TEXT_VA_ && va<TEXT_VA_+TEXT_SZ_; }

/* ------------------------------------------------------------------ relocs */
typedef struct { int64_t v; size_t k; } Sleb;
static int64_t sleb(const uint8_t *p, uint64_t n, uint64_t *pos){
    int64_t result=0; int shift=0; uint64_t i=*pos;
    for (;;){
        if (i>=n) return 0;
        uint8_t b=p[i++];
        result |= (int64_t)(b & 0x7f) << shift;
        shift += 7;
        if (!(b & 0x80)) break;
        if (shift > 63) break;
    }
    if (shift < 64 && (p[*pos + ((shift-1)/7)] & 0x40)) result |= -((int64_t)1 << shift);
    *pos = i;
    return result;
}

static uint64_t RS_N_ALLOC_;

static void rs_add(uint64_t off, uint64_t add){
    if (RS_N_ALLOC_==RS_CAP_){ RS_CAP_ = RS_CAP_? RS_CAP_*2 : (1u<<20);
        RS_ = xrealloc(RS_, RS_CAP_*sizeof(RelSlot)); }
    RS_[RS_N_ALLOC_].off=off; RS_[RS_N_ALLOC_].add=add; RS_N_ALLOC_++;
}

static void parse_rela_region(uint64_t off, uint64_t sz, uint64_t ent, int is_rel){
    if (!off || !sz) return;
    const uint8_t *p = D_+off;
    for (uint64_t k=0;k+ent<=sz;k+=ent){
        uint64_t ro = rd64(p+k), ri = rd64(p+k+8);
        uint32_t type = (uint32_t)(ri & 0xffffffff);
        if (type != R_AARCH64_RELATIVE_) continue;
        uint64_t add;
        if (is_rel){
            const uint8_t *q = va_ptr(ro, 8);
            if (!q) continue;
            add = rd64(q);
        } else add = rd64(p+k+16);
        rs_add(ro, add);
    }
}
/* LLD Android packed (APS2/APR2): flags — 1:grouped-by-info, 2:grouped-by-offset-delta,
   4:grouped-by-addend, 8:group-has-addend. Read order: gsize, gflags, [offdelta if 2],
   [info if 1], [group_addend if 8&4]. Per-reloc: [offdelta if !2][info if !1][addend sleb if 8&&!4] */
static void parse_aps2(uint64_t off, uint64_t sz, int is_rel_magic){
    (void)is_rel_magic;
    if (!off || sz < 8) return;
    const uint8_t *p = D_+off;
    if (memcmp(p, "APS2", 4) && memcmp(p, "APR2", 4)) return; /* not packed (legacy plain) */
    uint64_t pos = 4;
    int64_t count = sleb(p, sz, &pos);
    int64_t offset = sleb(p, sz, &pos);
    int64_t addend = 0;
    int64_t done = 0;
    while (done < count){
        if (pos >= sz) break;
        int64_t gsize = sleb(p, sz, &pos);
        int64_t gflags = sleb(p, sz, &pos);
        int64_t g_offdelta=0, g_info=0, g_addend=0;
        if (gflags & 2) g_offdelta = sleb(p, sz, &pos);
        if (gflags & 1) g_info     = sleb(p, sz, &pos);
        if ((gflags & 8) && (gflags & 4)) g_addend = sleb(p, sz, &pos);
        for (int64_t r=0; r<gsize && done<count; r++, done++){
            if (!(gflags & 2)) offset += sleb(p, sz, &pos);
            else               offset += g_offdelta;
            uint64_t info;
            if (gflags & 1) info = (uint64_t)g_info;
            else            info = (uint64_t)sleb(p, sz, &pos);
            uint64_t add = 0; int have_add = 0;
            if (gflags & 8){
                if (gflags & 4) addend = g_addend;
                else            addend += sleb(p, sz, &pos);
                add = (uint64_t)addend; have_add = 1;
            } else {
                const uint8_t *q = va_ptr((uint64_t)offset, 8);
                if (q){ add = rd64(q); have_add = 1; }
            }
            if (have_add && (uint32_t)(info & 0xffffffff) == R_AARCH64_RELATIVE_)
                rs_add((uint64_t)offset, add);
        }
    }
}
static void parse_relr(uint64_t off, uint64_t sz){
    if (!off || sz<8) return;
    const uint8_t *p=D_+off;
    uint64_t base=0;
    for (uint64_t k=0; k+8<=sz; k+=8){
        uint64_t w = rd64(p+k);
        if (w & 1){
            base = w & ~(uint64_t)1;
            const uint8_t *q = va_ptr(base,8);
            if (q) rs_add(base, rd64(q));
        } else {
            for (int b=0;b<63;b++){
                if (w & ((uint64_t)1<<b)){
                    uint64_t a = base + (uint64_t)(b+1)*8;
                    const uint8_t *q = va_ptr(a,8);
                    if (q) rs_add(a, rd64(q));
                }
            }
        }
    }
}

static int rel_cmp(const void *a, const void *b){
    const RelSlot *x=a,*y=b;
    if (x->off<y->off) return -1; if (x->off>y->off) return 1;
    if (x->add<y->add) return -1; if (x->add>y->add) return 1; return 0;
}

/* binary search: exact slot at off (first) */
static const RelSlot *rs_at(uint64_t off){
    uint64_t lo=0, hi=NRS_;
    while (lo<hi){ uint64_t m=(lo+hi)/2; if (RS_[m].off<off) lo=m+1; else hi=m; }
    if (lo<NRS_ && RS_[lo].off==off) return &RS_[lo];
    return NULL;
}
/* value of relocated slot (addend) or 0 */
static int rs_val(uint64_t off, uint64_t *out){
    const RelSlot *s = rs_at(off);
    if (!s) return 0;
    *out = s->add; return 1;
}
/* find first index with add == val (slots sorted by off, so linear scan w/ early
   exit is too slow; build reverse check via binary search on demand instead) */

/* ---------------------------------------------------------------- dynamic */
typedef struct {
    uint64_t rela, relasz, relaent;
    uint64_t rel, relsz, relent;
    uint64_t arela, arelasz, arel, arelsz;
    uint64_t relr, relrsz, relrent;
    uint64_t initarray;
} DynInfo;
static DynInfo DY_;

static void parse_dynamic(void){
    /* find PT_DYNAMIC via section ".dynamic" or phdr type 2 */
    const uint8_t *p=NULL; uint64_t sz=0;
    for (int i=0;i<NPH_;i++) if (PH_[i].p_type==2){ p=D_+PH_[i].p_offset; sz=PH_[i].p_filesz; break; }
    if (!p){ for (int i=0;i<NSH_;i++) if (!strcmp(sec_name(&SH_[i]),".dynamic")){ p=D_+SH_[i].sh_offset; sz=SH_[i].sh_size; } }
    if (!p) return;
    for (uint64_t k=0;k+16<=sz;k+=16){
        uint64_t tag=rd64(p+k), val=rd64(p+k+8);
        if (tag==0) break;
        switch (tag){
            case 7:  DY_.rela=val; break;    /* DT_RELA */
            case 8:  DY_.relasz=val; break;
            case 9:  DY_.relaent=val; break;
            case 17: DY_.rel=val; break;     /* DT_REL */
            case 18: DY_.relsz=val; break;
            case 19: DY_.relent=val; break;
            case 0x6fffe000: case 0x60000011: DY_.arela=val; break;  /* DT_ANDROID_RELA */
            case 0x6fffe001: case 0x60000012: DY_.arelasz=val; break;
            case 0x6ffff000: case 0x6000000f: DY_.arel=val; break;   /* DT_ANDROID_REL  */
            case 0x6ffff001: case 0x60000010: DY_.arelsz=val; break;
            case 36: DY_.relr=val; break;    /* DT_RELR */
            case 35: DY_.relrsz=val; break;
            case 37: DY_.relrent=val; break;
            case 25: DY_.initarray=val; break;
            default: break;
        }
    }
}

static void load_relocations(void){
    parse_rela_region(va2off(DY_.rela), DY_.relasz, DY_.relaent?:24, 0);
    parse_rela_region(va2off(DY_.rel),  DY_.relsz,  DY_.relent?:16, 1);
    parse_aps2(va2off(DY_.arela), DY_.arelasz, 1);
    parse_aps2(va2off(DY_.arel),  DY_.arelsz, 0);
    /* raw fallback: DT_ANDROID_RELA(SZ) may point at plain (non-"APS2") data:
       Android legacy packed also exists without magic; only magic form handled */
    parse_relr(va2off(DY_.relr), DY_.relrsz);
    qsort(RS_, RS_N_ALLOC_, sizeof(RelSlot), rel_cmp);
    NRS_ = RS_N_ALLOC_;
    /* dedup same off+add */
    uint64_t w=0;
    for (uint64_t i=0;i<NRS_;i++){
        if (w && RS_[i].off==RS_[w-1].off && RS_[i].add==RS_[w-1].add) continue;
        RS_[w++]=RS_[i];
    }
    NRS_=w;
}

/* all slots with add==val: iterate — need aux index sorted by add */
static RelSlot *RSA_; static uint64_t NRSA_; /* sorted by (add,off) */
static int rsa_cmp(const void*a,const void*b){
    const RelSlot *x=a,*y=b;
    if (x->add<y->add) return -1; if (x->add>y->add) return 1;
    if (x->off<y->off) return -1; if (x->off>y->off) return 1; return 0;
}
static void build_rsa(void){
    RSA_ = xmalloc(NRS_*sizeof(RelSlot));
    memcpy(RSA_, RS_, NRS_*sizeof(RelSlot));
    qsort(RSA_, NRS_, sizeof(RelSlot), rsa_cmp);
    NRSA_ = NRS_;
}
/* iterate slots whose add==val: returns [lo,hi) range in RSA_ */
static int rsa_find(uint64_t val, uint64_t *lo, uint64_t *hi){
    uint64_t l=0,h=NRSA_;
    while (l<h){ uint64_t m=(l+h)/2; if (RSA_[m].add<val) l=m+1; else h=m; }
    uint64_t a=l; h=NRSA_;
    while (l<h){ uint64_t m=(l+h)/2; if (RSA_[m].add<=val) l=m+1; else h=m; }
    *lo=a; *hi=l;
    return a<l;
}
/* ========================================================================== */
/* eh_frame: function boundary recovery (.eh_frame_hdr table + full CIE/FDE)  */
/* ========================================================================== */

static FuncRange *FR_; static uint64_t NFR_, CFR_;

static void fr_add(uint64_t s, uint64_t e){
    if (CFR_==NFR_){ NFR_ = NFR_? NFR_*2 : (1u<<19); FR_ = xrealloc(FR_, NFR_*sizeof(FuncRange)); }
    FR_[CFR_].start=s; FR_[CFR_].end=e; CFR_++;
}

static uint64_t eh_frame_addr(void){ return I_EHFRAME>=0 ? SH_[I_EHFRAME].sh_addr : 0; }

/* read an encoded eh pointer value. returns 1 on success */
static int read_enc_ptr(const uint8_t *p, uint64_t n, uint64_t *pos,
                        uint8_t enc, uint64_t base_pcrel, uint64_t base_datarel,
                        uint64_t *out)
{
    uint8_t fmt = enc & 0x0f, app = enc & 0xf0;
    int64_t v=0; uint64_t u=0; int is_signed=0;
    switch (fmt){
        case 0x00: /* absptr */ if (*pos+8>n) return 0; u=rd64(p+*pos); *pos+=8; break;
        case 0x01: { /* uleb128 */ uint64_t r=0; int sh=0;
            for (;;){ if (*pos>=n) return 0; uint8_t b=p[(*pos)++];
                r |= (uint64_t)(b&0x7f)<<sh; sh+=7; if (!(b&0x80)) break; }
            u=r; break; }
        case 0x02: /* udata2 */ if (*pos+2>n) return 0; u=rd32(p+*pos)&0xffff; *pos+=2; break;
        case 0x03: /* udata4 */ if (*pos+4>n) return 0; u=rd32(p+*pos); *pos+=4; break;
        case 0x04: /* udata8 */ if (*pos+8>n) return 0; u=rd64(p+*pos); *pos+=8; break;
        case 0x09: { /* sleb128 */ int64_t r=0; int sh=0;
            for (;;){ if (*pos>=n) return 0; uint8_t b=p[(*pos)++];
                r |= (int64_t)(b&0x7f)<<sh; sh+=7; if (!(b&0x80)) break; }
            if (sh<64 && (p[(*pos)-1] & 0x40)) r |= -((int64_t)1<<sh);
            v=r; is_signed=1; break; }
        case 0x0a: /* sdata2 */ if (*pos+2>n) return 0; v=(int16_t)(rd32(p+*pos)&0xffff); *pos+=2; is_signed=1; break;
        case 0x0b: /* sdata4 */ if (*pos+4>n) return 0; v=(int32_t)rd32(p+*pos); *pos+=4; is_signed=1; break;
        case 0x0c: /* sdata8 */ if (*pos+8>n) return 0; v=(int64_t)rd64(p+*pos); *pos+=8; is_signed=1; break;
        default: return 0;
    }
    uint64_t raw = is_signed ? (uint64_t)v : u;
    switch (app){
        case 0x00: *out = raw; break;                       /* DW_EH_PE_absptr */
        case 0x10: *out = base_pcrel + raw; break;          /* pcrel */
        case 0x30: *out = base_datarel + raw; break;        /* datarel */
        case 0x50: *out = 0; break;                         /* aligned — unsupported */
        default:   *out = 0; break;
    }
    return 1;
}

static void parse_eh_frame_hdr(void){
    if (I_EHFRAMEHDR < 0) return;
    const Elf64_Shdr *s = &SH_[I_EHFRAMEHDR];
    const uint8_t *p = D_ + s->sh_offset; uint64_t n = s->sh_size;
    if (n < 12) return;
    uint64_t hdr_va = s->sh_addr;
    if (p[0] != 1) return; /* version */
    uint8_t eptr_enc = p[1], cnt_enc = p[2], tab_enc = p[3];
    if (eptr_enc == 0xff || cnt_enc == 0xff || tab_enc == 0xff) return;
    uint64_t pos = 4, v;
    if (!read_enc_ptr(p, n, &pos, eptr_enc, hdr_va + pos, hdr_va, &v)) return;
    uint64_t cnt = 0;
    if (!read_enc_ptr(p, n, &pos, cnt_enc, hdr_va + pos, hdr_va, &v)) return;
    cnt = v;
    if (!cnt || cnt > 2000000) return;
    if (pos + cnt * 8 > n) return;
    for (uint64_t k=0;k<cnt;k++){
        uint64_t iloc, fptr;
        if (!read_enc_ptr(p,n,&pos,tab_enc, hdr_va+pos, hdr_va, &iloc)) break;
        if (!read_enc_ptr(p,n,&pos,tab_enc, hdr_va+pos, hdr_va, &fptr)) break;
        /* extent unknown here; will be patched by .eh_frame walk / next-start */
        if (in_text(iloc)) fr_add(iloc, 0);
    }
}

/* full .eh_frame walk. CIE cache by (section pos). */
static void parse_eh_frame(void){
    if (I_EHFRAME < 0) return;
    const Elf64_Shdr *s = &SH_[I_EHFRAME];
    const uint8_t *p = D_ + s->sh_offset; uint64_t n = s->sh_size;
    uint64_t eh_va = s->sh_addr;
    if (n < 8) return;

    /* collect CIEs first: pos -> fde_encoding */
    typedef struct { uint64_t pos; uint8_t enc; uint64_t datarel_base; } Cie;
    Cie *cies=NULL; uint64_t nc=0, capc=0;
    uint64_t pos = 0;
    while (pos + 4 <= n){
        uint64_t len = rd32(p+pos);
        if (len == 0) break;
        if (len == 0xffffffff) break;
        uint64_t after = pos + 4 + len;
        if (after > n) break;
        uint32_t id = (uint32_t)rd32(p+pos+4);
        if (id == 0){
            /* CIE */
            uint64_t q = pos + 8;
            if (q < n && p[q] != 1){ pos = after; continue; } /* version */
            q++;
            /* augmentation string */
            char aug[16]; int ai=0;
            while (q<n && p[q]){ if (ai<15) aug[ai++]=(char)p[q]; q++; }
            aug[ai]=0; q++;
            /* code align uleb, data align sleb, ret reg uleb(1 in v4) */
            while (q<n && (p[q] & 0x80)) q++; q++;
            { int sh=0; while (q<n){ int8_t b=(int8_t)p[q++]; if (!(b & 0x80)) break; } (void)sh; }
            { while (q<n && (p[q] & 0x80)) q++; q++; }
            uint8_t fde_enc = 0x1b; /* default pcrel|sdata4 */
            uint64_t dr_base = eh_va;
            if (aug[0]=='z'){
                uint64_t alen=0; int sh=0;
                while (q<n){ uint8_t b=p[q++]; alen |= (uint64_t)(b&0x7f)<<sh; sh+=7; if (!(b&0x80)) break; }
                uint64_t aend = q + alen;
                for (int i=1; aug[i]; i++){
                    if (aug[i]=='L'){ if (q<n) q++; }
                    else if (aug[i]=='P'){ if (q<n){ uint8_t pe=p[q++]; uint64_t tmp; read_enc_ptr(p,n,&q,pe, eh_va+q, dr_base, &tmp);} }
                    else if (aug[i]=='R'){ if (q<n) fde_enc=p[q++]; }
                    else if (aug[i]=='D'){ /* datarel base flag for FDE pcrel calc */ }
                    else break;
                }
                q = aend;
            }
            if (capc==nc){ capc = capc? capc*2:4096; cies=xrealloc(cies,capc*sizeof(Cie)); }
            cies[nc].pos=pos; cies[nc].enc=fde_enc; cies[nc].datarel_base=dr_base; nc++;
        }
        pos = after;
    }
    /* FDE pass */
    pos = 0;
    uint64_t fallback_dr = eh_va; /* per-file datarel candidate */
    while (pos + 4 <= n){
        uint64_t len = rd32(p+pos);
        if (len == 0) break;
        if (len == 0xffffffff) break;
        uint64_t after = pos + 4 + len;
        if (after > n) break;
        uint32_t id = (uint32_t)rd32(p+pos+4);
        if (id != 0){
            uint64_t cie_pos = pos + 4 - id;
            const Cie *cie = NULL;
            for (uint64_t i=0;i<nc;i++) if (cies[i].pos==cie_pos){ cie=&cies[i]; break; }
            uint8_t enc = cie ? cie->enc : 0x1b;
            uint64_t q = pos + 8;
            uint64_t iloc=0;
            int ok=0;
            if ((enc & 0xf0) == 0x30){
                /* datarel: try multiple bases; accept one that lands in text */
                uint64_t bases[3]; int nb=0;
                bases[nb++] = eh_va;
                if (I_EHFRAMEHDR>=0) bases[nb++] = SH_[I_EHFRAMEHDR].sh_addr;
                bases[nb++] = 0;
                for (int b=0;b<nb && !ok;b++){
                    uint64_t qq=q, val=0;
                    if (read_enc_ptr(p,n,&qq,enc, eh_va+q, bases[b], &val) && in_text(val)){
                        iloc=val; ok=1; fallback_dr=bases[b];
                    }
                }
            } else {
                uint64_t qq=q, val=0;
                if (read_enc_ptr(p,n,&qq,enc, eh_va+q, fallback_dr, &val)){ iloc=val; ok=in_text(iloc); }
                if (!ok){ /* retry with 0 base for abs */
                    qq=q; if (read_enc_ptr(p,n,&qq,enc, eh_va+q, 0, &val)){ iloc=val; ok=in_text(iloc);} }
            }
            if (ok){
                /* range: udata4/sdata4 following, per same size class as enc fmt */
                uint8_t fmt = enc & 0x0f;
                uint64_t rq = q;
                uint64_t dummy;
                read_enc_ptr(p,n,&rq,enc, eh_va+q, fallback_dr, &dummy); /* skip iloc */
                uint64_t range=0;
                if (fmt==0x03 || fmt==0x0b || fmt==0x09){ if (rq+4<=n){ range=rd32(p+rq);} }
                else if (fmt==0x0a || fmt==0x04){ if (rq+8<=n){ range=rd64(p+rq);} }
                else if (fmt==0x02 || fmt==0x0c){ if (rq+2<=n){ range=rd32(p+rq)&0xffff;} }
                fr_add(iloc, range ? iloc+range : 0);
            }
        }
        pos = after;
    }
    free(cies);
}

/* merge + sort function ranges */
static int fr_cmp(const void*a,const void*b){
    const FuncRange *x=a,*y=b;
    if (x->start<y->start) return -1; if (x->start>y->start) return 1; return 0;
}
static void build_funcs(void){
    parse_eh_frame_hdr();
    parse_eh_frame();
    qsort(FR_, CFR_, sizeof(FuncRange), fr_cmp);
    /* merge duplicates keeping max extent */
    uint64_t w=0;
    for (uint64_t i=0;i<CFR_;i++){
        if (w && FR_[i].start==FR_[w-1].start){
            if (FR_[i].end > FR_[w-1].end) FR_[w-1].end=FR_[i].end;
        } else FR_[w++]=FR_[i];
    }
    CFR_=w;
    /* fill missing extents with next start */
    for (uint64_t i=0;i<CFR_;i++)
        if (!FR_[i].end || FR_[i].end>TEXT_VA_+TEXT_SZ_)
            FR_[i].end = (i+1<CFR_ && FR_[i+1].start<=TEXT_VA_+TEXT_SZ_) ? FR_[i+1].start : TEXT_VA_+TEXT_SZ_;
    /* arrays */
    FS_ = xmalloc(CFR_*sizeof(uint64_t));
    FE_ = xmalloc(CFR_*sizeof(uint64_t));
    FK_ = xmalloc(CFR_*sizeof(int));
    for (uint64_t i=0;i<CFR_;i++){ FS_[i]=FR_[i].start; FE_[i]=FR_[i].end; FK_[i]=FK_NORMAL; }
    NFS_=CFR_;
    free(FR_); FR_=NULL; CFR_=NFR_=0;
}

/* greatest index with FS_[i] <= va ; -1 if none */
static int64_t func_idx(uint64_t va){
    if (!NFS_) return -1;
    uint64_t lo=0, hi=NFS_;
    while (lo<hi){ uint64_t m=(lo+hi)/2; if (FS_[m]<=va) lo=m+1; else hi=m; }
    return (int64_t)lo-1;
}
static uint64_t func_of(uint64_t va){
    int64_t i=func_idx(va);
    if (i<0) return 0;
    uint64_t f=FS_[i];
    return f;
}
static int is_func_start(uint64_t va){
    uint64_t lo=0, hi=NFS_;
    while (lo<hi){ uint64_t m=(lo+hi)/2; if (FS_[m]<va) lo=m+1; else hi=m; }
    return lo<NFS_ && FS_[lo]==va;
}
/* first function start strictly greater than va */
static uint64_t next_start(uint64_t va){
    uint64_t lo=0, hi=NFS_;
    while (lo<hi){ uint64_t m=(lo+hi)/2; if (FS_[m]<=va) lo=m+1; else hi=m; }
    return lo<NFS_ ? FS_[lo] : 0;
}
static uint64_t func_end_of(uint64_t f){
    uint64_t lo=0, hi=NFS_;
    while (lo<hi){ uint64_t m=(lo+hi)/2; if (FS_[m]<f) lo=m+1; else hi=m; }
    if (lo<NFS_ && FS_[lo]==f) return FE_[lo];
    return 0;
}
/* ========================================================================== */
/* ARM64 decoders, string search, code-ref index, xref heat, vtable runs      */
/* ========================================================================== */

static int64_t sext_(int64_t v,int bits){ int64_t m=1LL<<(bits-1); return (v^m)-m; }

/* instruction accessors relative to .text */
static uint32_t insn_at(uint64_t va){
    if (!in_text(va) || va & 3) return 0;
    const uint8_t *q = va_ptr(va,4);
    return q ? (uint32_t)rd32(q) : 0;
}
#define INSN(off) (*(const uint32_t *)(const void *)(TEXT_P_ + (off)))

static int dec_adrp(uint32_t i, uint64_t pc, int *rd, uint64_t *page){
    if ((i & 0x9F000000) != 0x90000000) return 0;
    *rd = i & 0x1f;
    uint64_t lo = (i>>29)&3, hi=(i>>5)&0x7ffff;
    int64_t simm = sext_((int64_t)((hi<<2)|lo),21);
    *page = (pc & ~(uint64_t)0xfff) + ((uint64_t)simm<<12);
    return 1;
}
/* ADD Xd, Xn, #imm{,LSL #12} (64-bit, flags clear) */
static int dec_add64(uint32_t i,int*rn,int*rd,uint64_t*imm){
    if ((i & 0xFF800000) != 0x91000000) return 0;
    if (((i>>22)&3) > 1) return 0;
    uint64_t i12=(i>>10)&0xfff;
    if (((i>>22)&3)==1) i12<<=12;
    *rd=i&0x1f; *rn=(i>>5)&0x1f; *imm=i12;
    return 1;
}
/* SUB imm 64 */
static int dec_sub64(uint32_t i,int*rn,int*rd,uint64_t*imm){
    if ((i & 0xFF800000) != 0xD1000000) return 0;
    if (((i>>22)&3) > 1) return 0;
    uint64_t i12=(i>>10)&0xfff;
    if (((i>>22)&3)==1) i12<<=12;
    *rd=i&0x1f; *rn=(i>>5)&0x1f; *imm=i12;
    return 1;
}
/* LDR Xt, [Xn, #imm] (imm scaled *8) */
static int dec_ldr64(uint32_t i,int*rn,int*rt,uint64_t*imm){
    if ((i & 0xFFC00000) != 0xF9400000) return 0;
    *imm=((i>>10)&0xfff)*8; *rn=(i>>5)&0x1f; *rt=i&0x1f; return 1;
}
static int dec_ldr32(uint32_t i,int*rn,int*rt,uint64_t*imm){
    if ((i & 0xFFC00000) != 0xB9400000) return 0;
    *imm=((i>>10)&0xfff)*4; *rn=(i>>5)&0x1f; *rt=i&0x1f; return 1;
}
static int dec_ldrb(uint32_t i,int*rn,int*rt,uint64_t*imm){
    if ((i & 0xFFC00000) != 0x39400000) return 0;
    *imm=((i>>10)&0xfff); *rn=(i>>5)&0x1f; *rt=i&0x1f; return 1;
}
static int dec_str64(uint32_t i,int*rn,int*rt,uint64_t*imm){
    if ((i & 0xFFC00000) != 0xF9000000) return 0;
    *imm=((i>>10)&0xfff)*8; *rn=(i>>5)&0x1f; *rt=i&0x1f; return 1;
}
static int dec_str32(uint32_t i,int*rn,int*rt,uint64_t*imm){
    if ((i & 0xFFC00000) != 0xB9000000) return 0;
    *imm=((i>>10)&0xfff)*4; *rn=(i>>5)&0x1f; *rt=i&0x1f; return 1;
}
static int dec_strb(uint32_t i,int*rn,int*rt,uint64_t*imm){
    if ((i & 0xFFC00000) != 0x39000000) return 0;
    *imm=((i>>10)&0xfff); *rn=(i>>5)&0x1f; *rt=i&0x1f; return 1;
}
static uint64_t dec_bl(uint32_t i,uint64_t pc){
    if ((i & 0xFC000000) != 0x94000000) return 0;
    int64_t imm=sext_((int64_t)(i & 0x03ffffff),26);
    return pc + ((uint64_t)imm<<2);
}
static uint64_t dec_b(uint32_t i,uint64_t pc){
    if ((i & 0x7C000000) != 0x14000000) return 0;
    int64_t imm=sext_((int64_t)(i & 0x03ffffff),26);
    return pc + ((uint64_t)imm<<2);
}
static int is_bcond(uint32_t i){ return (i & 0xFF000010) == 0x54000000; }
static int is_cbzcbnz(uint32_t i){ return (i & 0x7E000000) == 0x34000000; }
static int is_tbztbnz(uint32_t i){ return (i & 0x7E000000) == 0x36000000; }
static int is_blr(uint32_t i){ return (i & 0xFFFFFC1F) == 0xD63F0000 && ((i>>5)&0x1f)!=31; }
static int is_br(uint32_t i){ return (i & 0xFFFFFC1F) == 0xD61F0000 && ((i>>5)&0x1f)!=31; }
static int is_ret(uint32_t i){ return (i & 0xFFFFFC1F) == 0xD65F0000 || i==0xD65F03C0; }
/* MOV Xd, Xm (ORR Xd,XZR,Xm) */
static int dec_mov64(uint32_t i,int*rd,int*rm){
    if ((i & 0xFFE0FC00) != 0xAA000000) return 0;
    if (((i>>10)&0x3f) != 0) return 0;         /* shift amount 0 */
    if (((i>>5)&0x1f) != 0x1f) return 0;       /* Rn = XZR */
    *rd = i & 0x1f; *rm = (i>>16)&0x1f; return 1;
}
/* SUB SP, SP, #imm */
static int is_sub_sp(uint32_t i){
    int rn,rd; uint64_t imm;
    if (!dec_sub64(i,&rn,&rd,&imm)) return 0;
    return rn==31 && rd==31;
}
/* STP X29,X30,[SP,#-N]! / [SP,#off] */
static int is_stp_x29x30_pre(uint32_t i){ return (i & 0xFFC07FFF) == 0xA9807BFD; }
static int is_stp_x29x30_off(uint32_t i){ return (i & 0xFFC07FFF) == 0xA9007BFD; }
static int is_prologue_at(uint64_t f){
    uint32_t i0=insn_at(f), i1=insn_at(f+4);
    if (is_stp_x29x30_pre(i0) || is_stp_x29x30_pre(i1)) return 1;
    if (i0==0xD503233F /* PACIASP */) return 1;
    if (is_sub_sp(i0) || is_sub_sp(i1)) return 1;
    if (is_stp_x29x30_off(i0)) return 1;
    return 0;
}

/* ------------------------------------------------------- string searching */
/* find literal VAs containing `needle` (needle may be substring). wide=1 →
   UTF-16LE. fills vout[] (dedup), returns count. Also reports literal_start. */
typedef struct { uint64_t lit; uint64_t hit; } StrHit; /* hit = where needle matched */

static int str_find_all(const char *needle, int wide, int substring_ok,
                        StrHit **out_hits, uint64_t *out_n)
{
    uint64_t nlen = strlen(needle);
    if (!nlen || nlen>512) { *out_hits=NULL; *out_n=0; return 0; }
    uint8_t *pat = xmalloc(nlen*2+4);
    uint64_t plen=0;
    if (wide){
        for (uint64_t i=0;i<nlen;i++){ pat[plen++]= (uint8_t)needle[i]; pat[plen++]=0; }
    } else {
        for (uint64_t i=0;i<nlen;i++) pat[plen++]=(uint8_t)needle[i];
    }
    StrHit *hits=NULL; uint64_t nh=0, cap=0;
    for (int si=0; si<NSH_; si++){
        const Elf64_Shdr *s=&SH_[si];
        if (s->sh_type!=SHT_PROGBITS_ || (s->sh_flags & SHF_EXEC_) || !s->sh_addr || !s->sh_size) continue;
        if (s->sh_flags & SHF_EXEC_) { /* skip .plt/.text (may hold refs, not literals) */ }
        const uint8_t *sd = D_ + s->sh_offset;
        uint64_t ssz = s->sh_size;
        uint64_t step = wide?2:1;
        if (ssz < plen) continue;
        for (uint64_t o=0; o+plen<=ssz; o+=step){
            if (memcmp(sd+o, pat, plen)!=0) continue;
            /* must be terminator-terminated unless substring_ok picked mid-literal */
            uint64_t litoff;
            /* walk back to terminator */
            if (wide){
                uint64_t q=o;
                while (q>=2){ if (sd[q-2]==0 && sd[q-1]==0) break; q-=2; }
                litoff=q;
            } else {
                uint64_t q=o;
                while (q>=1 && sd[q-1]!=0) q--;
                litoff=q;
            }
            /* needle must be at literal start OR substring_ok */
            if (!substring_ok && litoff != o) continue;
            /* needle must be followed by terminator (exact literal = needle+\0)
               unless substring_ok */
            if (!substring_ok){
                uint64_t e=o+plen;
                if (wide){ if (e+2<=ssz && !(sd[e]==0 && sd[e+1]==0)) continue; }
                else { if (e<ssz && sd[e]!=0) continue; }
            }
            if (nh==cap){ cap=cap?cap*2:32; hits=xrealloc(hits,cap*sizeof(StrHit)); }
            hits[nh].lit = s->sh_addr + litoff;
            hits[nh].hit = s->sh_addr + o;
            nh++;
            if (nh>20000) break;
        }
    }
    /* dedup by lit */
    uint64_t w=0;
    for (uint64_t i=0;i<nh;i++){
        int dup=0;
        for (uint64_t j=0;j<w;j++) if (hits[j].lit==hits[i].lit){dup=1;break;}
        if (!dup) hits[w++]=hits[i];
    }
    nh=w;
    free(pat);
    *out_hits=hits; *out_n=nh;
    return (int)nh;
}
static void str_free(StrHit *h){ free(h); }

/* -------------------------------------------------- code-ref index build */
static int ref_cmp(const void*a,const void*b){
    const RefSite *x=a,*y=b;
    if (x->tgt<y->tgt) return -1; if (x->tgt>y->tgt) return 1;
    if (x->pc<y->pc) return -1; if (x->pc>y->pc) return 1; return 0;
}
static void build_refs(void){
    uint64_t cap = TEXT_SZ_/4 + 16;
    REF_  = xmalloc(cap*sizeof(RefSite)); NREF_=0;
    REFL_ = xmalloc(cap*sizeof(RefSite)); NREFL_=0;
    for (uint64_t off=0; off+4<=TEXT_SZ_; off+=4){
        uint32_t i1 = INSN(off);
        if ((i1 & 0x9F000000) != 0x90000000) continue;  /* ADRP */
        int rd1; uint64_t page;
        if (!dec_adrp(i1, TEXT_VA_+off, &rd1, &page)) continue;
        for (int j=1;j<=6;j++){
            if (off + j*4 + 4 > TEXT_SZ_) break;
            uint32_t i2 = INSN(off + j*4);
            int rn,rd,rt; uint64_t imm;
            if (dec_add64(i2,&rn,&rd,&imm) && rn==rd1){
                if (NREF_<cap){ REF_[NREF_].tgt=page+imm; REF_[NREF_].pc=TEXT_VA_+off; NREF_++; }
                break;
            }
            if (dec_ldr64(i2,&rn,&rt,&imm) && rn==rd1){
                if (NREFL_<cap){ REFL_[NREFL_].tgt=page+imm; REFL_[NREFL_].pc=TEXT_VA_+off; NREFL_++; }
                break;
            }
        }
    }
    qsort(REF_, NREF_, sizeof(RefSite), ref_cmp);
    qsort(REFL_, NREFL_, sizeof(RefSite), ref_cmp);
}
/* all ref sites to exact va (REF_) */
static uint64_t refs_to(uint64_t va, RefSite **arr){
    uint64_t lo=0,hi=NREF_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (REF_[m].tgt<va) lo=m+1; else hi=m;}
    uint64_t a=lo; hi=NREF_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (REF_[m].tgt<=va) lo=m+1; else hi=m;}
    if (a>=lo){ *arr=NULL; return 0; }
    *arr = REF_+a; return lo-a;
}
static uint64_t refsL_to(uint64_t va, RefSite **arr){
    uint64_t lo=0,hi=NREFL_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (REFL_[m].tgt<va) lo=m+1; else hi=m;}
    uint64_t a=lo; hi=NREFL_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (REFL_[m].tgt<=va) lo=m+1; else hi=m;}
    if (a>=lo){ *arr=NULL; return 0; }
    *arr = REFL_+a; return lo-a;
}

/* --------------------------------------------------------- BL/B call map */
static int cs_cmp(const void*a,const void*b){
    const CallSite *x=a,*y=b;
    if (x->tgt<y->tgt) return -1; if (x->tgt>y->tgt) return 1;
    if (x->pc<y->pc) return -1; if (x->pc>y->pc) return 1; return 0;
}
static void build_calls(void){
    uint64_t cap = TEXT_SZ_/4 + 16;
    BL_ = xmalloc(cap*sizeof(CallSite)); NBL_=0;
    for (uint64_t off=0; off+4<=TEXT_SZ_; off+=4){
        uint32_t i1=INSN(off);
        uint64_t pc=TEXT_VA_+off;
        uint64_t t = dec_bl(i1,pc);
        if (!t) t = dec_b(i1,pc);
        if (!t || !in_text(t)) continue;
        if (NBL_<cap){ BL_[NBL_].pc=pc; BL_[NBL_].tgt=t; NBL_++; }
    }
    qsort(BL_, NBL_, sizeof(CallSite), cs_cmp);
}
/* callers of target (sites sorted by pc) */
static uint64_t callers_of(uint64_t tgt, CallSite **arr){
    uint64_t lo=0,hi=NBL_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (BL_[m].tgt<tgt) lo=m+1; else hi=m;}
    uint64_t a=lo; hi=NBL_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (BL_[m].tgt<=tgt) lo=m+1; else hi=m;}
    if (a>=lo){ *arr=NULL; return 0; }
    *arr=BL_+a; return lo-a;
}
static uint64_t heat_of(uint64_t tgt){
    CallSite *a; return callers_of(tgt,&a);
}

/* --------------------------------------------------------- vtable runs */
static int vt_slotcmp(const void*a,const void*b){
    uint64_t x=*(const uint64_t*)a, y=*(const uint64_t*)b;
    if (x<y) return -1; if (x>y) return 1; return 0;
}
static int slot_is_codeptr(uint64_t slot){
    uint64_t v;
    if (!rs_val(slot,&v)) return 0;
    if (!in_text(v) || !is_func_start(v)) return 0;
    return 1;
}
static int vt_skip_section(const Elf64_Shdr *s){
    const char *n=sec_name(s);
    if (!sec_writable(s)) return 1;
    if (!strncmp(n,".got",4)) return 1;
    if (!strncmp(n,".init_array",11)) return 1;
    if (!strncmp(n,".fini_array",11)) return 1;
    if (!strncmp(n,".preinit",8)) return 1;
    if (!strncmp(n,".dynamic",8)) return 1;
    if (!strncmp(n,".rela",5)) return 1;
    if (!strncmp(n,".rel.",5)) return 1;
    if (!strncmp(n,".relr",5)) return 1;
    if (!strncmp(n,".jcr",4)) return 1;
    if (!strncmp(n,".igot.plt",9)) return 1;
    if (!strncmp(n,".data.rel.ro",12)){
        /* .data.rel.ro is THE vtable home — keep. exclude .got inside? handled */
        return 0;
    }
    if (!strncmp(n,".data",5) || !strncmp(n,".bss",4)) return 0;
    return 0;
}
static void build_vtables(void){
    /* collect consecutive code-slot runs in writable alloc sections */
    uint64_t cap=1u<<20;
    uint64_t *slots=xmalloc(cap*sizeof(uint64_t)); uint64_t ns=0;
    uint64_t *runs =xmalloc(cap*sizeof(uint64_t)); uint64_t nr=0;
    for (int si=0; si<NSH_; si++){
        const Elf64_Shdr *s=&SH_[si];
        if (s->sh_type!=SHT_PROGBITS_ || (s->sh_flags & SHF_EXEC_) || !s->sh_addr || !s->sh_size) continue;
        if (vt_skip_section(s)) continue;
        uint64_t a = s->sh_addr, e = a + s->sh_size;
        for (uint64_t v=(a+7)&~(uint64_t)7; v+8<=e; v+=8){
            if (!slot_is_codeptr(v)) continue;
            if (ns==cap){ cap*=2; slots=xrealloc(slots,cap*8); runs=xrealloc(runs,cap*8); }
            uint64_t runstart;
            if (ns && slots[ns-1]+8==v) runstart = runs[ns-1];
            else runstart = v;
            slots[ns]=v; runs[ns]=runstart; ns++;
        }
    }
    /* filter runs shorter than 4; build sorted (slot, run-start) pairs */
    typedef struct { uint64_t slot, run; } SR;
    SR *sr = xmalloc(ns*sizeof(SR));
    { uint64_t w=0; uint64_t i=0;
      while (i<ns){
        uint64_t j=i;
        while (j<ns && runs[j]==runs[i]) j++;
        if (j-i >= 4){
            for (uint64_t k=i;k<j;k++){ sr[w].slot=slots[k]; sr[w].run=runs[i]; w++; }
        }
        i=j;
      }
      ns=w;
    }
    qsort(sr, ns, sizeof(SR), vt_slotcmp);
    VT_SLOT_ = xmalloc(ns*8); VT_RUN_=xmalloc(ns*8);
    for (uint64_t k=0;k<ns;k++){ VT_SLOT_[k]=sr[k].slot; VT_RUN_[k]=sr[k].run; }
    free(sr);
    VT_N_=ns; VT_NR_=ns;
    free(slots); free(runs);
}
/* vtable base for slot (0 if not in a run) */
static uint64_t vt_base_of_slot(uint64_t slot){
    uint64_t lo=0,hi=VT_N_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (VT_SLOT_[m]<slot) lo=m+1; else hi=m;}
    if (lo<VT_N_ && VT_SLOT_[lo]==slot) return VT_RUN_[lo];
    return 0;
}
/* ========================================================================== */
/* Function classification, outlined-log resolution, thunk validation,        */
/* Path A (name → table → thunk) and Path B (string → function)               */
/* ========================================================================== */

static void classify_all(void){
    for (uint64_t i=0;i<NFS_;i++){
        uint64_t f=FS_[i], e=FE_[i];
        uint64_t sz = (e>f)? e-f : 16;
        if (sz<=16){
            /* bridge? single B / B.cond */
            uint32_t i0=insn_at(f);
            uint64_t t = dec_b(i0,f);
            if (t) { FK_[i]=FK_BRIDGE; continue; }
            if (is_bcond(i0)||is_cbzcbnz(i0)||is_tbztbnz(i0)) { FK_[i]=FK_BRIDGE; continue; }
        }
        if (sz<=0x60){
            /* log thunk: ADRP+ADD→string, MOV W3,#imm, MOV X0,X30, tail B */
            int has_str=0, has_w3=0, has_x30=0, tail_b=0;
            for (uint64_t a=f; a+4<=e && a<f+0x60; a+=4){
                uint32_t w=insn_at(a);
                if ((w & 0x9F000000)==0x90000000){
                    int rd; uint64_t pg;
                    if (dec_adrp(w,a,&rd,&pg)){
                        for (int j=1;j<=2;j++){
                            uint32_t w2=insn_at(a+j*4);
                            int rn,rd2,rt; uint64_t imm;
                            if (dec_add64(w2,&rn,&rd2,&imm)&&rn==rd){
                                const Elf64_Shdr *s=sec_of(pg+imm);
                                if (s && !(s->sh_flags & SHF_EXEC_)) has_str=1;
                            }
                            if (dec_ldr64(w2,&rn,&rt,&imm)&&rn==rd){
                                const Elf64_Shdr *s=sec_of(pg+imm);
                                if (s && !(s->sh_flags & SHF_EXEC_)) has_str=1;
                            }
                        }
                    }
                }
                if ((w & 0xFFE0001F) == 0x52800003){ has_w3=1; } /* MOVZ W3,#imm */
                if (w == 0xAA1E03E0){ has_x30=1; }               /* MOV X0,X30 */
                if (dec_b(w,a)) tail_b=1;
            }
            if (has_str && (has_w3||has_x30) && tail_b){ FK_[i]=FK_LOGTHUNK; continue; }
            if (sz<=0x60) { FK_[i]=FK_TINY; continue; }
        }
        FK_[i]=FK_NORMAL;
    }
}
static int fkind_of(uint64_t f){
    uint64_t lo=0,hi=NFS_;
    while (lo<hi){uint64_t m=(lo+hi)/2; if (FS_[m]<f) lo=m+1; else hi=m;}
    if (lo<NFS_ && FS_[lo]==f) return FK_[lo];
    return -1;
}

/* ------------------------------------------------------------------ */
/* Outlined resolution.                                                 */
/* A "piece" (log-thunk / bridge / tiny fn) that references our string   */
/* belongs to a parent function, found by:                              */
/*   (a) tail-B chain: piece's B lands mid-function of parent           */
/*   (b) unique BL caller of the piece                                  */
/* ------------------------------------------------------------------ */
#define MAX_PIECES 64
#define MAX_CANDS  64
typedef struct { uint64_t fn; int votes; int kind; uint64_t sz; } ParentCand;

static void add_parent(ParentCand *c, int *nc, uint64_t fn){
    for (int i=0;i<*nc;i++) if (c[i].fn==fn){ c[i].votes++; return; }
    if (*nc<MAX_CANDS){
        c[*nc].fn=fn; c[*nc].votes=1; c[*nc].kind=fkind_of(fn);
        uint64_t e=func_end_of(fn);
        c[*nc].sz=(e>fn)?e-fn:0;
        (*nc)++;
    }
}

static void outlined_scan(uint64_t piece, uint64_t *seen, int *nseen, ParentCand *c, int *nc){
    if (*nseen>=MAX_PIECES) return;
    for (int i=0;i<*nseen;i++) if (seen[i]==piece) return;
    seen[(*nseen)++]=piece;
    uint64_t e=func_end_of(piece);
    if (!e || e<=piece) return;
    /* (a) tail B chain: any B in piece */
    for (uint64_t a=piece; a+4<=e; a+=4){
        uint32_t w=insn_at(a);
        uint64_t t=dec_b(w,a);
        if (!t) continue;
        if (!in_text(t)) continue;
        uint64_t g=func_of(t);
        if (!g) continue;
        if (t!=g){
            /* mid-function landing → parent */
            if (g!=piece) add_parent(c,nc,g);
        } else if (t==g && g!=piece && (fkind_of(g)==FK_BRIDGE||fkind_of(g)==FK_LOGTHUNK||fkind_of(g)==FK_TINY)){
            outlined_scan(g,seen,nseen,c,nc);
        }
    }
    /* (b) callers via BL/B — hop up through outlined pieces */
    {
        CallSite *cs; uint64_t n=callers_of(piece,&cs);
        if (n>=1 && n<=MAX_CANDS*4){
            uint64_t fns[MAX_CANDS*4]; int nf=0;
            for (uint64_t k=0;k<n;k++){
                uint64_t g=func_of(cs[k].pc);
                if (!g||g==piece) continue;
                int dup=0;
                for (int q=0;q<nf;q++) if (fns[q]==g){dup=1;break;}
                if (!dup && nf<MAX_CANDS*4) fns[nf++]=g;
            }
            if (nf==1){
                int k2=fkind_of(fns[0]);
                if (k2==FK_BRIDGE||k2==FK_LOGTHUNK||k2==FK_TINY)
                    outlined_scan(fns[0],seen,nseen,c,nc);
                else add_parent(c,nc,fns[0]);
            } else if (nf>1){
                for (int q=0;q<nf;q++){
                    int k2=fkind_of(fns[q]);
                    if (k2==FK_BRIDGE||k2==FK_LOGTHUNK||k2==FK_TINY)
                        outlined_scan(fns[q],seen,nseen,c,nc);
                    else add_parent(c,nc,fns[q]);
                }
            }
        }
    }
}

/* Resolve a string-ref's true containing function.
   Returns 0 on failure. Fills cands (may be >1: ambiguous). */
static uint64_t resolve_ref_function(uint64_t ref_pc, ParentCand *cands, int *nc_)
{
    *nc_=0;
    uint64_t f = func_of(ref_pc);
    if (!f) return 0;
    int k = fkind_of(f);
    if (k==FK_NORMAL) { add_parent(cands,nc_,f); return f; }
    /* outlined piece → find parents */
    uint64_t seen[MAX_PIECES]; int nseen=0;
    outlined_scan(f, seen, &nseen, cands, nc_);
    if (!*nc_){ add_parent(cands,nc_,f); return f; } /* fallback: itself */
    if (*nc_==1) return cands[0].fn;
    /* multiple: prefer NORMAL kind with most votes, then size */
    int best=0;
    for (int i=1;i<*nc_;i++){
        int ki=cands[i].kind, kb=cands[best].kind;
        int si=(ki==FK_NORMAL), sb=(kb==FK_NORMAL);
        if (si!=sb){ if (si) best=i; continue; }
        if (cands[i].votes!=cands[best].votes){ if (cands[i].votes>cands[best].votes) best=i; continue; }
        if (cands[i].sz>cands[best].sz) best=i;
    }
    return cands[best].fn;
}

/* ------------------------------------------------------------ exec thunks */
/* Z_Construct<UFunction> stub shape: LDR Xn,[X0,#0]; CBZ; …; single BL; RET */
static int looks_like_zc_stub(uint64_t f){
    uint64_t e=func_end_of(f);
    if (!e||e<=f||e-f>0x50) return 0;
    int ldr_x0_0=0, cbz=0, bl=0;
    for (uint64_t a=f;a+4<=e;a+=4){
        uint32_t w=insn_at(a);
        int rn,rt; uint64_t imm;
        if (dec_ldr64(w,&rn,&rt,&imm)&&rn==0&&imm==0) ldr_x0_0=1;
        if (is_cbzcbnz(w)) cbz=1;
        if (dec_bl(w,a)) bl++;
    }
    return ldr_x0_0 && cbz && bl<=2;
}
/* exec thunk: prologue + >=1 call + FFrame usage via X1 or parked X19..X28 */
static int is_exec_thunk(uint64_t f){
    if (!is_func_start(f)) return 0;
    if (looks_like_zc_stub(f)) return 0;
    uint64_t e=func_end_of(f);
    if (!e||e<=f) return 0;
    uint64_t lim = e; if (lim > f+0x200) lim=f+0x200;
    int saw_call=0, fframe=0;
    int parked[32]; memset(parked,0,sizeof(parked)); /* parked[Xn]=1 → holds Frame (X1) */
    for (uint64_t a=f;a+4<=lim;a+=4){
        uint32_t w=insn_at(a);
        if (dec_bl(w,a) || is_blr(w)) saw_call=1;
        int rd,rm;
        if (dec_mov64(w,&rd,&rm) && rm==1 && rd>=19 && rd<=28) parked[rd]=1;
        int rn,rt; uint64_t imm;
        /* LDR/STR [X1, #off] */
        if (dec_ldr64(w,&rn,&rt,&imm)||dec_str64(w,&rn,&rt,&imm)||
            dec_ldr32(w,&rn,&rt,&imm)||dec_str32(w,&rn,&rt,&imm)){
            if (rn==1 && imm>=0x8 && imm<=0x90) fframe=1;
            if (rn>=19&&rn<=28 && parked[rn] && imm>=0x8 && imm<=0x90) fframe=1;
        }
    }
    return saw_call && fframe;
}

/* orig impl of a thunk: last non-hot BL target */
static uint64_t thunk_orig(uint64_t f, uint64_t *bl_last, uint64_t heat_min){
    uint64_t e=func_end_of(f);
    if (!e||e<=f) return 0;
    uint64_t best=0, last=0;
    for (uint64_t a=f;a+4<=e;a+=4){
        uint32_t w=insn_at(a);
        uint64_t t=dec_bl(w,a);
        if (!t||!in_text(t)||!is_func_start(t)) continue;
        last=t;
        if (heat_of(t) <= heat_min) best=t;   /* later non-hot wins */
    }
    if (bl_last) *bl_last=last;
    return best;
}

/* ------------------------------------------------------------------ Path A */
/* name → {Name,Func} table slot → thunk (+orig) */
typedef struct {
    uint64_t thunk, orig;
    uint64_t slot;      /* name slot addr */
    int      buddy;     /* +8 / -8 */
    int      wide;      /* name encoding */
} PathARes;
static int path_a(const char *name, PathARes *out, int out_cap){
    int nres=0;
    for (int wide=0; wide<=1; wide++){
        StrHit *hits; uint64_t nh;
        if (!str_find_all(name, wide, 0, &hits,&nh)) continue;
        for (uint64_t h=0;h<nh && nres<out_cap;h++){
            uint64_t sv=hits[h].lit;
            uint64_t lo,hi;
            if (!rsa_find(sv,&lo,&hi)) continue;
            for (uint64_t k=lo;k<hi && nres<out_cap;k++){
                uint64_t slot=RSA_[k].off;   /* slot whose addend == name */
                const Elf64_Shdr *ss=sec_of(slot);
                if (!ss || !sec_writable(ss)) continue;
                for (int buddy=8; buddy>=-8; buddy-=16){
                    if (!buddy) continue;
                    uint64_t tv;
                    if (!rs_val(slot+buddy,&tv)) continue;
                    if (!in_text(tv) || !is_func_start(tv)) continue;
                    if (!is_exec_thunk(tv)) continue;
                    /* dedup */
                    int dup=0;
                    for (int q=0;q<nres;q++) if (out[q].thunk==tv){dup=1;break;}
                    if (dup) continue;
                    out[nres].thunk=tv;
                    out[nres].orig=thunk_orig(tv,NULL,100);
                    out[nres].slot=slot;
                    out[nres].buddy=buddy;
                    out[nres].wide=wide;
                    nres++;
                }
            }
        }
        str_free(hits);
        if (nres) break;   /* prefer wide/ascii order: ascii found first wins */
    }
    return nres;
}

/* ------------------------------------------------------------------ Path B */
/* string → code refs → containing function (outlined-aware) */
typedef struct {
    uint64_t fn;
    int      votes;
    int      direct;   /* refs landing inside fn directly */
} PathBCand;
static int path_b(const char *needle, int wide, PathBCand *cands, int max_c, int *found_hit)
{
    StrHit *hits; uint64_t nh;
    if (found_hit) *found_hit=0;
    if (!str_find_all(needle, wide, 1, &hits,&nh)) { return 0; }
    if (found_hit && nh) *found_hit=1;
    int nc=0;
    for (uint64_t h=0;h<nh;h++){
        RefSite *rs; uint64_t nr_ = refs_to(hits[h].lit, &rs);
        /* indirect: GOT-slot loads (ADRP+LDR of a reloc'd pointer to the string) */
        uint64_t slo,shi;
        if (rsa_find(hits[h].lit,&slo,&shi)){
            for (uint64_t q=slo;q<shi;q++){
                RefSite *rl; uint64_t nl_=refsL_to(RSA_[q].off,&rl);
                for (uint64_t w=0;w<nl_;w++){
                    ParentCand pc[64]; int npc=0;
                    uint64_t fn=resolve_ref_function(rl[w].pc,pc,&npc);
                    int direct=(func_of(rl[w].pc)==fn);
                    int idx=-1;
                    for (int i=0;i<nc;i++) if (cands[i].fn==fn){idx=i;break;}
                    if (idx<0 && nc<max_c){ idx=nc++; cands[idx].fn=fn; cands[idx].votes=0; cands[idx].direct=0; }
                    if (idx>=0){ cands[idx].votes++; if (direct) cands[idx].direct++; }
                }
                /* also plain ADD of the slot address */
                RefSite *ra_; uint64_t na_=refs_to(RSA_[q].off,&ra_);
                for (uint64_t w=0;w<na_;w++){
                    ParentCand pc[64]; int npc=0;
                    uint64_t fn=resolve_ref_function(ra_[w].pc,pc,&npc);
                    int direct=(func_of(ra_[w].pc)==fn);
                    int idx=-1;
                    for (int i=0;i<nc;i++) if (cands[i].fn==fn){idx=i;break;}
                    if (idx<0 && nc<max_c){ idx=nc++; cands[idx].fn=fn; cands[idx].votes=0; cands[idx].direct=0; }
                    if (idx>=0){ cands[idx].votes++; if (direct) cands[idx].direct++; }
                }
            }
        }
        for (uint64_t r=0;r<nr_;r++){
            ParentCand pc[64]; int npc=0;
            uint64_t fn = resolve_ref_function(rs[r].pc, pc, &npc);
            int direct = (func_of(rs[r].pc)==fn);
            int idx=-1;
            for (int i=0;i<nc;i++) if (cands[i].fn==fn){idx=i;break;}
            if (idx<0 && nc<max_c){ idx=nc++; cands[idx].fn=fn; cands[idx].votes=0; cands[idx].direct=0; }
            if (idx>=0){ cands[idx].votes++; if (direct) cands[idx].direct++; }
        }
    }
    str_free(hits);
    return nc;
}
/* ========================================================================== */
/* Special modes: cksz, gis, namepool, vtbase, chain, vftnext, callee,       */
/* encpatch, cvar, anchor                                                     */
/* ========================================================================== */

/* ---------------------------------------------------------------- cksz mode
 * User's "pattern 4": anchor string → callers → BL histogram of
 * (spec-ctor, next BL) — ConstructAbilitySpec + GiveAbility.               */
typedef struct { uint64_t ctor, next_bl; int count, weighted; } CKHist;
static int cksz_score(const uint64_t fn)
{
    uint64_t e=func_end_of(fn);
    if (!e||e<=fn) return 0;
    if (fn < TEXT_VA_ || fn - TEXT_VA_ + 0x400 > TEXT_SZ_) return 0;
    uint64_t lim = e; if (lim > fn+0x400) lim=fn+0x400;
    int hits=0;
    for (uint64_t a=fn; a+4<=lim; a+=4){
        uint32_t w=insn_at(a);
        int rn,rt; uint64_t imm;
        if (dec_str64(w,&rn,&rt,&imm)){
            if (rn==0 && (imm==0x10 || imm==0x20 || imm==0x8 || imm==0x18)) hits++;
        } else if (dec_str32(w,&rn,&rt,&imm)){
            if (rn==0 && (imm==0xC || imm==0x18 || imm==0x1C || imm==0x10 || imm==0x14)) hits++;
        } else if (dec_strb(w,&rn,&rt,&imm)){
            if (rn==0 && (imm==0x28 || imm==0x29)) hits++;
        }
        if (hits>=4) return hits;
    }
    return hits;
}
static uint64_t cksz_find(const char *anchor, int wide, uint64_t *give_ab)
{
    StrHit *hits; uint64_t nh;
    if (give_ab) *give_ab=0;
    if (!str_find_all(anchor, wide, 1, &hits,&nh) || !nh){ str_free(hits); return 0; }
    /* callers (containing functions) of the anchor string */
    uint64_t callers[256]; int ncallers=0;
    for (uint64_t h=0;h<nh;h++){
        RefSite *rs; uint64_t nr_=refs_to(hits[h].lit,&rs);
        for (uint64_t r=0;r<nr_;r++){
            ParentCand pc[64]; int npc=0;
            uint64_t fn=resolve_ref_function(rs[r].pc,pc,&npc);
            int dup=0;
            for (int q=0;q<ncallers;q++) if (callers[q]==fn){dup=1;break;}
            if (!dup && ncallers<256) callers[ncallers++]=fn;
        }
    }
    str_free(hits);
    if (!ncallers) return 0;
    CKHist hist[128]; int nhist=0;
    for (int c=0;c<ncallers;c++){
        uint64_t fs=callers[c];
        uint64_t fe_=func_end_of(fs);
        if (!fe_||fe_<=fs) continue;
        uint64_t fend=fe_; if (fend > fs+0x4000) fend=fs+0x4000;
        for (uint64_t a=fs; a+8<=fend; a+=4){
            uint32_t w=insn_at(a);
            uint64_t t=dec_bl(w,a);
            if (!t||!in_text(t)||!is_func_start(t)||t==fs) continue;
            int sc=cksz_score(t);
            if (sc<4) continue;
            uint64_t next_bl=0;
            for (uint64_t b=a+4; b+4<=a+32 && b+4<=fend; b+=4){
                uint64_t t2=dec_bl(insn_at(b),b);
                if (t2){ next_bl=t2; break; }
            }
            int dup=0;
            for (int i=0;i<nhist;i++)
                if (hist[i].ctor==t && hist[i].next_bl==next_bl){ hist[i].count++; hist[i].weighted+=sc; dup=1; break; }
            if (!dup && nhist<128){ hist[nhist].ctor=t; hist[nhist].next_bl=next_bl;
                hist[nhist].count=1; hist[nhist].weighted=sc; nhist++; }
        }
    }
    int best=-1;
    for (int i=0;i<nhist;i++)
        if (best<0 || hist[i].count>hist[best].count ||
           (hist[i].count==hist[best].count && hist[i].weighted>hist[best].weighted)) best=i;
    if (best<0) return 0;
    if (give_ab) *give_ab=hist[best].next_bl;
    return hist[best].ctor;
}

/* ---------------------------------------------------------------- gis mode
 * "AllowCommandletRendering" → FEngineLoop::PreInit sub-fn → the two 1-byte
 * .bss globals it sets = GIsClient / GIsServer (VARIABLES, not functions).  */
typedef struct { uint64_t gis_client, gis_server, fn; } GisRes;
static int gis_find(const char *anchor, GisRes *out)
{
    memset(out,0,sizeof(*out));
    PathBCand cands[64]; int nc = path_b(anchor, 1, cands, 64, NULL);
    if (!nc) nc = path_b(anchor, 0, cands, 64, NULL);
    if (!nc) return 0;
    /* pick the fn with most votes */
    int best=0;
    for (int i=1;i<nc;i++) if (cands[i].votes>cands[best].votes) best=i;
    uint64_t f=cands[best].fn;
    out->fn=f;
    uint64_t e=func_end_of(f);
    if (!e||e<=f) return 0;
    uint64_t g1=0,g2=0;
    for (uint64_t a=f;a+4<=e;a+=4){
        uint32_t w=insn_at(a);
        if ((w & 0x9F000000)!=0x90000000) continue;   /* ADRP */
        int rd; uint64_t pg;
        if (!dec_adrp(w,a,&rd,&pg)) continue;
        for (int j=1;j<=3;j++){
            uint32_t w2=insn_at(a+j*4);
            int rn,rd2=0,rt; uint64_t imm;
            uint64_t g=0;
            if (dec_add64(w2,&rn,&rd2,&imm)&&rn==rd) g=pg+imm;
            else if (dec_ldrb(w2,&rn,&rt,&imm)&&rn==rd) g=pg+imm;
            else if (dec_ldr64(w2,&rn,&rt,&imm)&&rn==rd) g=pg+imm;
            if (!g) continue;
            const Elf64_Shdr *s=sec_of(g);
            if (!s || !sec_writable(s) || (s->sh_flags&SHF_EXEC_)) continue;
            /* bool-sized global: next insn STRB/LDRB w/ base from this ADRP */
            int is_bool=0;
            for (int k=j;k<=j+2;k++){
                uint32_t w3=insn_at(a+k*4);
                int rn3,rt3; uint64_t im3;
                if (dec_strb(w3,&rn3,&rt3,&im3)||dec_ldrb(w3,&rn3,&rt3,&im3)) is_bool=1;
                if ((w3 & 0xFF80001F)==0x52800000) is_bool=1; /* MOVZ Wd,#imm (bool const) */
            }
            if (!is_bool) continue;
            if (g1==g && g1) continue;
            if (!g1) g1=g;
            else if (!g2 && g!=g1) g2=g;
        }
    }
    if (g1 && g2){
        /* GIsClient first-assigned order: PreInit sets client then server-ish;
           heuristics: GIsClient == the one ALSO referenced by UWorld::IsClient
           (a 2-insn fn: ADRP+LDRB of GIsClient; RET). Check tiny fns. */
        uint64_t score1=0, score2=0;
        for (int side=0;side<2;side++){
            uint64_t g = side? g2:g1;
            RefSite *rs; uint64_t n=refs_to(g,&rs); (void)n;
            RefSite *rl; uint64_t nl=refsL_to(g,&rl);
            /* count LDRB-style accesses via REFL_ proximity not tracked; use
               tiny-fn heuristic: find functions <=16B that reference g */
            uint64_t refs=0;
            for (uint64_t q=0;q<nl;q++){
                uint64_t fn2=func_of(rl[q].pc);
                if (fn2){
                    uint64_t e2=func_end_of(fn2);
                    if (e2>fn2 && e2-fn2<=16) refs++;
                }
            }
            if (side) score2=refs; else score1=refs;
        }
        if (score2>score1){ out->gis_client=g2; out->gis_server=g1; }
        else { out->gis_client=g1; out->gis_server=g2; }
        return 2;
    }
    if (g1){ out->gis_client=g1; return 1; }
    return 0;
}

/* ----------------------------------------------------------- namepool mode
 * MobileDumper-7 style anchors. UE5: FNamePool ctor refs EName strings;
 * NamePoolData = .bss global passed as `this` to the ctor. UE4.21: GNames. */
typedef struct {
    uint64_t pool;          /* NamePoolData / GNames global   */
    uint64_t ctor;          /* FNamePool ctor / FName init fn */
    uint64_t accessor;      /* GetNamePool()-like accessor    */
    uint64_t err_fn;        /* fn referencing ERROR_NAME_SIZE_EXCEEDED */
    int      layout;        /* 0=TNameEntryArray 1=FNamePool  */
} NamePoolRes;

typedef struct { uint64_t g; int add, ldr, addrtaken, x0bl; } GlobAcc;
static void np_harvest(uint64_t fn, GlobAcc *ga, int *ng, int cap){
    uint64_t e=func_end_of(fn);
    if (!e||e<=fn) return;
    for (uint64_t a=fn;a+4<=e;a+=4){
        uint32_t w=insn_at(a);
        if ((w & 0x9F000000)!=0x90000000) continue;
        int rd; uint64_t pg;
        if (!dec_adrp(w,a,&rd,&pg)) continue;
        for (int j=1;j<=3;j++){
            uint32_t w2=insn_at(a+j*4);
            int rn,rd2=0,rt; uint64_t imm;
            uint64_t g=0; int is_add=0,is_ldr=0;
            if (dec_add64(w2,&rn,&rd2,&imm)&&rn==rd){ g=pg+imm; is_add=1; }
            else if (dec_ldr64(w2,&rn,&rt,&imm)&&rn==rd){ g=pg+imm; is_ldr=1; }
            if (!g) continue;
            const Elf64_Shdr *s=sec_of(g);
            if (!s || !sec_writable(s)) continue;
            if (s->sh_type==SHT_NOBITS_ || 1){ /* prefer .bss but allow .data */
            }
            int idx=-1;
            for (int q=0;q<*ng;q++) if (ga[q].g==g){idx=q;break;}
            if (idx<0){ if (*ng>=cap) continue; idx=(*ng)++; ga[idx].g=g; ga[idx].add=ga[idx].ldr=ga[idx].addrtaken=ga[idx].x0bl=0; }
            ga[idx].add+=is_add; ga[idx].ldr+=is_ldr;
            /* address-taken into X0/X1 then BL within 3 insns (ctor placement-new) */
            if (is_add){
                int rd_ = rd2;
                if (rd_<=1){
                    for (int k=j;k<=j+2;k++){
                        uint32_t w3=insn_at(a+k*4);
                        if (dec_bl(w3,a+k*4)){ ga[idx].x0bl++; ga[idx].addrtaken++; break; }
                    }
                }
            }
        }
    }
}
static int namepool_find(NamePoolRes *out)
{
    memset(out,0,sizeof(*out));
    /* EName strings — ANSI literals (UE5 ctor) */
    static const char *enames[] = {"ByteProperty","IntProperty","ObjectProperty",
        "BoolProperty","FloatProperty","NameProperty","DelegateProperty",NULL};
    uint64_t ctor=0; int best_hits=-1;
    uint64_t ctor_fns[64]; int ncf=0;
    for (int i=0; enames[i]; i++){
        StrHit *hits; uint64_t nh;
        if (!str_find_all(enames[i], 0, 0, &hits,&nh)){ str_free(hits); continue; }
        for (uint64_t h=0;h<nh;h++){
            RefSite *rs; uint64_t nr_=refs_to(hits[h].lit,&rs);
            for (uint64_t r=0;r<nr_;r++){
                uint64_t fn=func_of(rs[r].pc);
                if (!fn) continue;
                /* count how many EName strings this fn references */
                int hits2=0;
                for (int q=0; enames[q]; q++){
                    StrHit *h2; uint64_t n2;
                    if (!str_find_all(enames[q],0,0,&h2,&n2)){ str_free(h2); continue; }
                    for (uint64_t z=0;z<n2 && !hits2;z++){
                        RefSite *r2; uint64_t m=refs_to(h2[z].lit,&r2);
                        for (uint64_t y=0;y<m;y++) if (func_of(r2[y].pc)==fn){hits2=1;break;}
                    }
                    str_free(h2);
                }
                if (hits2>best_hits){
                    best_hits=hits2; ctor=fn; ncf=0; ctor_fns[ncf++]=fn;
                } else if (hits2==best_hits && hits2>=3){
                    int dup=0; for (int q=0;q<ncf;q++) if (ctor_fns[q]==fn)dup=1;
                    if (!dup&&ncf<64) ctor_fns[ncf++]=fn;
                }
            }
        }
        str_free(hits);
    }
    if (ctor) for (int q=0;q<ncf;q++) if (ctor_fns[q]!=ctor){ /* union */ }
    out->ctor=ctor;
    if (!ctor) return 0;

    /* harvest globals from ctor + its callers */
    GlobAcc ga[256]; int ng=0;
    np_harvest(ctor, ga, &ng, 256);
    CallSite *cs; uint64_t nc=callers_of(ctor,&cs);
    for (uint64_t k=0;k<nc && k<64;k++){
        uint64_t cf=func_of(cs[k].pc);
        if (cf) np_harvest(cf, ga, &ng, 256);
    }
    /* UE5: pick global passed as X0 to ctor (x0bl) with max refs */
    uint64_t pool=0; int bestsc=-1;
    for (int i=0;i<ng;i++){
        if (!ga[i].x0bl) continue;
        int sc=ga[i].x0bl*8+ga[i].add+ga[i].ldr;
        if (sc>bestsc){ bestsc=sc; pool=ga[i].g; }
    }
    if (!pool){
        /* fallback: most-accessed writable global */
        for (int i=0;i<ng;i++){
            int sc=ga[i].add+ga[i].ldr+ga[i].addrtaken*2;
            if (sc>bestsc){ bestsc=sc; pool=ga[i].g; }
        }
    }
    out->pool=pool;
    out->layout=1;
    /* accessor: tiny fn ADRP+ADD pool; RET */
    if (pool){
        RefSite *rs; uint64_t n=refs_to(pool,&rs);
        for (uint64_t r=0;r<n;r++){
            uint64_t fn=func_of(rs[r].pc);
            if (!fn) continue;
            uint64_t e=func_end_of(fn);
            if (e>fn && e-fn<=0x10){
                int ok=0;
                for (uint64_t a=fn;a+4<=e;a+=4){
                    uint32_t w=insn_at(a);
                    if ((w & 0xFFE0FFFF)==0xAA0003E0) ok=1;  /* MOV X0,Xn */
                    if (is_ret(w)) { if(ok){ out->accessor=fn; } }
                }
            }
            if (out->accessor) break;
        }
    }
    /* ERROR_NAME_SIZE_EXCEEDED fn */
    {
        PathBCand cb[8]; int n2=path_b("ERROR_NAME_SIZE_EXCEEDED",1,cb,8,NULL);
        if (!n2) n2=path_b("ERROR_NAME_SIZE_EXCEEDED",0,cb,8,NULL);
        if (n2) out->err_fn=cb[0].fn;
    }
    return out->pool?1:0;
}

/* --------------------------------------------------------------- vtbase mode
 * UNetConnection: Tick anchor → function; find its vtable via "Tick is a
 * member slot" + vcall validation; report base + IsNetReady @ +0x330.      */
typedef struct {
    uint64_t tick_fn, vt_base;
    uint64_t isnetready_fn;    /* resolved from base+0x330 */
    int      nvcalls;
    int      found;
} VtBaseRes;

/* scan fn for vcall sites: LDR Xt,[Xobj] ... LDR Xm,[Xt,#imm] ... BLR Xm */
static int vcall_sites(uint64_t fn, uint64_t *offs_out, int max){
    uint64_t e=func_end_of(fn);
    if (!e||e<=fn) return 0;
    int n=0;
    for (uint64_t a=fn;a+4<=e;a+=4){
        uint32_t w=insn_at(a);
        int rn,rt; uint64_t imm0;
        /* LDR Xt, [Xobj]  (imm=0) */
        if (!dec_ldr64(w,&rn,&rt,&imm0) || imm0!=0) continue;
        int vt_reg=rt;
        for (int j=1;j<=6 && a+j*4+4<=e;j++){
            uint32_t w2=insn_at(a+j*4);
            int rn2,rt2; uint64_t imm2;
            if (dec_ldr64(w2,&rn2,&rt2,&imm2) && rn2==vt_reg){
                /* next: BLR rt2 within 6 insns */
                for (int k=j+1;k<=j+6 && a+k*4+4<=e;k++){
                    uint32_t w3=insn_at(a+k*4);
                    if (is_blr(w3) && (int)((w3>>5)&0x1f)==rt2){
                        if (n<max) offs_out[n++]=imm2;
                        k=99; break;
                    }
                    if (dec_bl(w3,a+k*4)) break; /* direct call interrupts */
                }
                break;
            }
            if (is_blr(w2)||is_br(w2)) break;
            if (dec_bl(w2,a+j*4)) break;
        }
        if (n>=max) break;
    }
    return n;
}
static int vtbase_find(const char *tick_anchor, VtBaseRes *out)
{
    memset(out,0,sizeof(*out));
    PathBCand cb[16]; int nc=path_b(tick_anchor,1,cb,16,NULL);
    if (!nc) nc=path_b(tick_anchor,0,cb,16,NULL);
    if (!nc) return 0;
    int best=0;
    for (int i=1;i<nc;i++) if (cb[i].votes>cb[best].votes) best=i;
    uint64_t tick=cb[best].fn;
    out->tick_fn=tick;
    /* vcall offsets inside Tick */
    uint64_t voffs[64]; int nv=vcall_sites(tick, voffs, 64);
    out->nvcalls=nv;
    /* candidate vtables = runs containing Tick */
    uint64_t lo,hi;
    if (!rsa_find(tick,&lo,&hi)) return 0;
    for (uint64_t k=lo;k<hi;k++){
        uint64_t slot=RSA_[k].off;
        uint64_t base=vt_base_of_slot(slot);
        if (!base) continue;
        /* validate: all discovered vcall offsets resolve to code slots */
        int okc=0;
        for (int i=0;i<nv;i++){
            uint64_t v;
            if (rs_val(base+voffs[i],&v) && in_text(v) && is_func_start(v)) okc++;
        }
        if (base && (nv==0 || okc*2>=nv*1)){
            out->vt_base=base;
            uint64_t f330;
            if (rs_val(base+0x330,&f330) && in_text(f330)) out->isnetready_fn=f330;
            out->found=1;
            return 1;
        }
    }
    return 0;
}

/* ---------------------------------------------------------------- chain mode
 * donor fn (via inner anchor) → scan MOV X0,X(19-28); BL f; B g           */
static int chain_scan(uint64_t donor, uint64_t *f, uint64_t *g){
    uint64_t e=func_end_of(donor);
    if (!e||e<=donor) return 0;
    for (uint64_t a=donor;a+8<=e;a+=4){
        uint32_t w=insn_at(a);
        int rd,rm;
        if (!dec_mov64(w,&rd,&rm)) continue;
        if (rd!=0) continue;                  /* X0 */
        if (rm<19||rm>28) continue;
        uint64_t t=dec_bl(insn_at(a+4),a+4);
        if (!t||!in_text(t)) continue;
        /* B g within 2 insns after BL */
        for (int j=2;j<=4;j++){
            uint64_t bg=dec_b(insn_at(a+j*4),a+j*4);
            if (bg && in_text(bg)){ *f=t; *g=bg; return 1; }
        }
    }
    return 0;
}

/* ---------------------------------------------------------------- encpatch */
static uint64_t encpatch_find(const char *anchor)
{
    PathBCand cb[16]; int nc=path_b(anchor,1,cb,16,NULL);
    if (!nc) nc=path_b(anchor,0,cb,16,NULL);
    if (!nc) return 0;
    int best=0;
    for (int i=1;i<nc;i++) if (cb[i].votes>cb[best].votes) best=i;
    uint64_t f=cb[best].fn;
    /* first cond-branch site in f that tests a local/stack flag == 1 */
    uint64_t e=func_end_of(f);
    if (!e||e<=f) return 0;
    for (uint64_t a=f;a+4<=e;a+=4){
        uint32_t w=insn_at(a);
        if ((w & 0x7F800000)==0x71000000){   /* SUBS imm / CMP Wn,#imm */
            if (((w>>5)&0x1f)==0x1f && (((w>>10)&0xfff)+1)==1){  /* CMP Xn,#1 */
                uint32_t w2=insn_at(a+4);
                if (is_bcond(w2) || is_cbzcbnz(w2)) return a+4;   /* patch site = branch */
            }
        }
        if (is_cbzcbnz(w)){
            /* CBZ on a register loaded from a bool param — patch site */
            return a;
        }
        if (a > f+0x400) break;
    }
    return 0;
}

/* ------------------------------------------------------------------ helpers */
#if 0
----------- (v7 new modes were appended below via patch) -----------
#endif

static uint64_t path_b_any(const char *s, PathBCand *best_c, int *nc_out, int prefer_big);

/* ========================================================================== */
/* v7 modes: execvslot, blsite, poolmember, dref                              */
/* ========================================================================== */

/* ------------------------------------------------------------- execvslot
 * native name → exec thunk ({Name,Func} table) → the LAST inlined-wrapper
 * virtual call  (LDR Xm,[Xk]; … LDR Xn,[Xm,#imm]; BLR Xn)  → vtable slot
 * offset imm → the _Implementation function at (winning vtable)+imm.
 *
 * On UE4.21/UE5.0 Android the generated RPC wrapper is inlined into the
 * exec thunk; _Validate and _Implementation are reached through vtable
 * slots of the owner class.  The LAST such vcall in the thunk is
 * _Implementation (Erbium's RemoveInventoryItem / …StateValue targets).  */
typedef struct { uint64_t off; int blr_pc; } ExecVCall;
static int exec_vcall_slots(uint64_t thunk, ExecVCall *out, int max)
{
    uint64_t e = func_end_of(thunk);
    if (!e || e <= thunk || e - thunk > 0x2000) return 0;
    int n = 0;
    for (uint64_t a = thunk; a + 4 <= e && n < max; a += 4) {
        uint32_t wb = insn_at(a);
        if (!is_blr(wb)) continue;
        int rd = (int)((wb >> 5) & 0x1f);          /* BLR Xrd */
        /* back-walk: slot load LDR rd,[Xb,#imm], then plain vtable load */
        int base_reg = -1; uint64_t slot_off = 0; int have_slot = 0, have_vt = 0;
        for (int j = 1; j <= 12; j++) {
            if (a < thunk + (uint64_t)(j*4)) break;
            uint32_t w2 = insn_at(a - (uint64_t)(j*4));
            if ((w2 & 0xFFC00000) == 0xF9400000) {          /* LDR Xt,[Xn,#imm] */
                int rn2 = (int)((w2 >> 5) & 0x1f);
                int rt2 = (int)(w2 & 0x1f);
                uint64_t imm = ((w2 >> 10) & 0xfff) * 8;
                if (!have_slot && rt2 == rd) {
                    have_slot = 1; base_reg = rn2; slot_off = imm;
                    continue;
                }
                if (have_slot && !have_vt && rt2 == base_reg && imm == 0) {
                    have_vt = 1; break;      /* LDR Xb,[Xk] — object vtable load */
                }
            }
        }
        if (have_slot && have_vt && slot_off) {
            if (n < max) { out[n].off = slot_off; out[n].blr_pc = (int)(a - thunk); n++; }
        }
    }
    return n;
}

typedef struct { uint64_t base, fn; int score; } VtCand;
static int execvslot_find(const char *name, int nth, uint64_t *fn_out,
                          uint64_t *base_out, uint64_t *off_out, int *idx_out,
                          char *how, size_t howcap)
{
    PathARes pa[8]; int n = path_a(name, pa, 8);
    if (!n) return 0;
    uint64_t thunk = 0, nslot = 0;
    for (int i = 0; i < n; i++)
        if (pa[i].buddy > 0) { thunk = pa[i].thunk; nslot = pa[i].slot; break; }
    if (!thunk) { thunk = pa[0].thunk; nslot = pa[0].slot; }
    ExecVCall vc[16]; int nv = exec_vcall_slots(thunk, vc, 16);
    if (!nv) return 0;
    if (nth < 1 || nth > nv) nth = nv;         /* default: LAST = _Impl */
    uint64_t off = vc[nth-1].off;

    /* enumerate vtable run bases; candidate = base where base+off holds a
     * relocated function start */
    static VtCand cands[4096]; int nc = 0;
    for (uint64_t i = 0; i < VT_N_ && nc < 4096; i++) {
        if (VT_SLOT_[i] != VT_RUN_[i]) continue;          /* run start only */
        uint64_t B = VT_SLOT_[i];
        uint64_t s = B + off;
        uint64_t v;
        if (!rs_val(s, &v)) continue;
        if (!in_text(v) || !is_func_start(v)) continue;
        if (vt_base_of_slot(s) != B) continue;
        int score = 0;
        /* heat: _Impl is only reached via BLR → no direct BL callers */
        CallSite *cs; uint64_t h = callers_of(v, &cs);
        if (h == 0) score += 10; else if (h <= 2) score += 5; else if (h > 8) score -= 6;
        /* neighbors: sibling virtuals should also be func starts */
        for (int d = -3; d <= 3; d++) {
            if (!d) continue;
            uint64_t v2;
            if (rs_val(s + (uint64_t)d*8, &v2) && in_text(v2) && is_func_start(v2))
                score += 1;
        }
        /* dedup same fn (derived vtables share impls) */
        int dup = 0;
        for (int q = 0; q < nc; q++)
            if (cands[q].fn == v) {
                if (score > cands[q].score ||
                    (score == cands[q].score && B < cands[q].base)) {
                    cands[q].base = B; cands[q].score = score;
                }
                dup = 1; break;
            }
        if (!dup) { cands[nc].base = B; cands[nc].fn = v; cands[nc].score = score; nc++; }
    }
    if (!nc) return 0;
    int best = 0;
    for (int i = 1; i < nc; i++)
        if (cands[i].score > cands[best].score ||
            (cands[i].score == cands[best].score && cands[i].base < cands[best].base))
            best = i;
    uint64_t fn = cands[best].fn, bridge = 0;
    /* resolve through 4-byte tail bridges (slot may hold a B-veneer) */
    for (int hop = 0; hop < 4; hop++) {
        uint64_t e2 = func_end_of(fn);
        if (!e2 || e2 <= fn || e2 - fn > 8) break;
        uint64_t t2 = dec_b(insn_at(fn), fn);
        if (!t2 || !in_text(t2)) break;
        bridge = fn; fn = t2;
    }
    *fn_out = fn; *base_out = cands[best].base; *off_out = off;
    *idx_out = (int)(off / 8);
    if (bridge)
        snprintf(how, howcap,
            "execvslot: %s → thunk 0x%llX vcall +0x%llX (vt #%d/%d); base 0x%llX idx %d (score %d, %d cands); slot bridge 0x%llX",
            name, (unsigned long long)thunk, (unsigned long long)off, nth, nv,
            (unsigned long long)cands[best].base, (int)(off/8), cands[best].score, nc,
            (unsigned long long)bridge);
    else
        snprintf(how, howcap,
            "execvslot: %s → thunk 0x%llX vcall +0x%llX (vt #%d/%d); base 0x%llX idx %d (score %d, %d cands)",
            name, (unsigned long long)thunk, (unsigned long long)off, nth, nv,
            (unsigned long long)cands[best].base, (int)(off/8), cands[best].score, nc);
    return 1;
}


/* resolve the _Implementation at a KNOWN vtable base + slot offset
 * (used by the batch majority re-vote) */
static uint64_t execvslot_at(uint64_t base, uint64_t off)
{
    uint64_t v;
    if (!rs_val(base + off, &v)) return 0;
    if (!in_text(v) || !is_func_start(v)) return 0;
    return v;
}

/* ------------------------------------------------------------------ blsite
 * anchor string → enclosing fn → the last BL *before* the log callsite
 * whose result is tested as bool (Erbium's FindListenCall: the E8 call to
 * Listen right before "LoadMap: failed to Listen(%s)").  Ref collection
 * mirrors path_b: direct ADRP+ADD, GOT-slot loads and slot-address ADDs. */
static uint64_t blsite_find(const char *anchor, uint64_t *bl_pc, uint64_t *bl_tgt,
                            uint64_t *enclosing)
{
    uint64_t f = path_b_any(anchor, NULL, NULL, 0);
    if (!f) return 0;
    uint64_t e = func_end_of(f);
    if (!e || e <= f) return 0;
    for (int wide = 1; wide >= 0; wide--) {
        StrHit *hits; uint64_t nh;
        if (!str_find_all(anchor, wide, 1, &hits, &nh) || !nh) { str_free(hits); continue; }
        for (uint64_t h = 0; h < nh; h++) {
            /* gather raw ref sites for this literal */
            static uint64_t sites[512]; int ns = 0;
            RefSite *rs; uint64_t n = refs_to(hits[h].lit, &rs);
            for (uint64_t k = 0; k < n && ns < 512; k++) sites[ns++] = rs[k].pc;
            uint64_t slo, shi;
            if (rsa_find(hits[h].lit, &slo, &shi)) {
                for (uint64_t q = slo; q < shi; q++) {
                    RefSite *rl; uint64_t nl = refsL_to(RSA_[q].off, &rl);
                    for (uint64_t w = 0; w < nl && ns < 512; w++) sites[ns++] = rl[w].pc;
                    RefSite *ra; uint64_t na = refs_to(RSA_[q].off, &ra);
                    for (uint64_t w = 0; w < na && ns < 512; w++) sites[ns++] = ra[w].pc;
                }
            }
            for (int k = 0; k < ns; k++) {
                uint64_t pc = sites[k];
                uint64_t piece = func_of(pc);
                uint64_t site = 0;
                if (piece == f) site = pc;
                else if (piece) {
                    for (int hop = 0; hop < 12 && !site; hop++) {
                        CallSite *cs; uint64_t m = callers_of(piece, &cs);
                        uint64_t nextp = 0;
                        for (uint64_t q = 0; q < m; q++) {
                            uint64_t cf = func_of(cs[q].pc);
                            if (cf == f) { site = cs[q].pc; break; }
                            int kk = fkind_of(cf);
                            if ((kk == FK_BRIDGE || kk == FK_LOGTHUNK || kk == FK_TINY) && !nextp) nextp = cf;
                        }
                        if (!site && !nextp) break;
                        if (!site) piece = nextp;
                    }
                }
                if (!site) {
                    /* (c) tail-B mid-landing: outlined cold piece whose B
                     * lands inside f — the landing point is our site */
                    uint64_t pe = piece ? func_end_of(piece) : 0;
                    if (piece && pe > piece) {
                        for (uint64_t a2 = piece; a2 + 4 <= pe; a2 += 4) {
                            uint64_t t2 = dec_b(insn_at(a2), a2);
                            if (t2 && t2 > f && t2 < e) { site = t2; break; }
                        }
                    }
                }
                if (!site || site < f || site >= e) continue;
                /* scan backwards from the site for the nearest BL to a
                 * function start whose result is bool-tested */
                for (int d = 1; d <= 48; d++) {
                    if (site < (uint64_t)d*4) break;
                    uint64_t a = site - (uint64_t)d*4;
                    uint64_t t = dec_bl(insn_at(a), a);
                    if (!t || !in_text(t) || !is_func_start(t)) continue;
                    if (func_of(a) != f) break;
                    uint32_t w2 = insn_at(a + 4);
                    int tested = is_cbzcbnz(w2) || is_bcond(w2) || is_tbztbnz(w2);
                    if (tested) {
                        *bl_pc = a; *bl_tgt = t; *enclosing = f;
                        str_free(hits);
                        return t;
                    }
                }
            }
        }
        str_free(hits);
    }
    return 0;
}

/* ----------------------------------------------------------------- blsite2
 * anchor log string → enclosing fn F (e.g. UEngine::LoadMap);
 * helper anchor → helper fn H (e.g. CreateNetDriver).  The wanted call
 * inside F is the BL whose target is a *caller of H* — e.g. the BL to
 * UWorld::Listen (Listen calls CreateNetDriver; LoadMap's basic blocks
 * are scrambled so "the BL before the log" is unreliable).  */
static uint64_t blsite2_find(const char *log_anchor, const char *helper_anchor,
                             uint64_t *bl_pc, uint64_t *bl_tgt, uint64_t *enclosing)
{
    uint64_t f = path_b_any(log_anchor, NULL, NULL, 0);
    if (!f) return 0;
    uint64_t e = func_end_of(f);
    if (!e || e <= f) return 0;
    uint64_t h = path_b_any(helper_anchor, NULL, NULL, 0);
    if (!h) return 0;
    /* set of functions that (transitively, through tiny tail-bridges)
     * call h — bridges hop BOTH directions */
    static uint64_t callset[8192]; int nc = 0;
    static uint64_t work[256]; int nw = 0;
    static uint64_t seen[512]; int nseen = 0;
    work[nw++] = h;
    while (nw > 0 && nc < 8192) {
        uint64_t cur = work[--nw];
        int dup = 0;
        for (int q = 0; q < nseen; q++) if (seen[q] == cur) { dup = 1; break; }
        if (dup || nseen < 512) seen[nseen++] = cur;
        if (dup) continue;
        CallSite *cs; uint64_t n = callers_of(cur, &cs);
        for (uint64_t k = 0; k < n && k < 4096; k++) {
            uint64_t g = func_of(cs[k].pc);
            if (!g || g == cur) continue;
            uint64_t e2 = func_end_of(g);
            int tiny = (e2 > g && e2 - g <= 0x18);
            if (tiny && nw < 250) { work[nw++] = g; continue; }  /* bridge: keep hopping up */
            int dup2 = 0;
            for (int q = 0; q < nc; q++) if (callset[q] == g) { dup2 = 1; break; }
            if (!dup2) callset[nc++] = g;
        }
        if (nw > 200) break;
    }
    if (!nc) return 0;
    /* scan F's BLs (nearest to the log-site first: find the site for ranking) */
    uint64_t logsite = 0;
    for (int wide = 1; wide >= 0 && !logsite; wide--) {
        StrHit *hits; uint64_t nh;
        if (!str_find_all(log_anchor, wide, 1, &hits, &nh) || !nh) { str_free(hits); continue; }
        for (uint64_t hh = 0; hh < nh && !logsite; hh++) {
            RefSite *rs; uint64_t nr_ = refs_to(hits[hh].lit, &rs);
            for (uint64_t r = 0; r < nr_ && !logsite; r++)
                if (func_of(rs[r].pc) == f) logsite = rs[r].pc;
            if (!logsite) {
                uint64_t slo, shi;
                if (rsa_find(hits[hh].lit, &slo, &shi)) {
                    for (uint64_t q = slo; q < shi && !logsite; q++) {
                        RefSite *rl; uint64_t nl = refsL_to(RSA_[q].off, &rl);
                        for (uint64_t w = 0; w < nl && !logsite; w++)
                            if (func_of(rl[w].pc) == f) logsite = rl[w].pc;
                    }
                }
            }
        }
        str_free(hits);
    }
    /* all BLs in f that target a member of callset */
    uint64_t best_pc = 0, best_t = 0; uint64_t bestd = ~0ull;
    for (uint64_t a = f; a + 4 <= e; a += 4) {
        uint64_t t = dec_bl(insn_at(a), a);
        if (!t || !in_text(t)) continue;
        /* resolve t through tiny bridges down to the real function */
        uint64_t tr = t;
        for (int hop = 0; hop < 4; hop++) {
            uint64_t e2 = func_end_of(tr);
            if (!e2 || e2 <= tr || e2 - tr > 0x10) break;
            uint64_t nt = 0;
            for (uint64_t a2 = tr; a2 + 4 <= e2; a2 += 4) {
                uint64_t b = dec_b(insn_at(a2), a2);
                if (b && in_text(b)) { nt = b; break; }
                uint64_t bl2 = dec_bl(insn_at(a2), a2);
                if (bl2 && in_text(bl2)) { nt = bl2; break; }
            }
            if (!nt || !is_func_start(nt)) break;
            tr = nt;
        }
        int hit = 0;
        for (int q = 0; q < nc; q++) if (callset[q] == tr || callset[q] == t) { hit = 1; break; }
        if (!hit) continue;
        uint64_t d = logsite ? (logsite > a ? logsite - a : a - logsite) : 0;
        if (d < bestd) { bestd = d; best_pc = a; best_t = t; }
    }
    if (best_t) {
        *bl_pc = best_pc; *bl_tgt = best_t; *enclosing = f;
        return best_t;
    }
    return 0;
}

/* --------------------------------------------------------------- dref scan
 * all code refs (ADRP + ADD/LDR64/LDR32/LDRB) to a data VA */
static int dref_scan(uint64_t target, uint64_t *pcs, int max)
{
    int n = 0;
    uint64_t page = target & ~(uint64_t)0xfff;
    uint64_t toff = target & 0xfff;
    for (uint64_t off = 0; off + 4 <= TEXT_SZ_ && n < max; off += 4) {
        uint32_t i1 = INSN(off);
        if ((i1 & 0x9F000000) != 0x90000000) continue;
        int rd1; uint64_t pg;
        if (!dec_adrp(i1, TEXT_VA_ + off, &rd1, &pg)) continue;
        if (pg != page) continue;
        for (int j = 1; j <= 6; j++) {
            if (off + (uint64_t)j*4 + 4 > TEXT_SZ_) break;
            uint32_t i2 = INSN(off + (uint64_t)j*4);
            int rn = (int)((i2 >> 5) & 0x1f);
            if (rn != rd1) continue;
            uint64_t imm = 0; int hit = 0;
            if ((i2 & 0xFFC00000) == 0x91000000) {          /* ADD imm */
                imm = (i2 >> 10) & 0xfff;
                if ((i2 >> 22) & 3) imm <<= 12;
                hit = 1;
            } else if ((i2 & 0xFFC00000) == 0xF9400000) {   /* LDR X */
                imm = ((i2 >> 10) & 0xfff) * 8; hit = 1;
            } else if ((i2 & 0xFFC00000) == 0xB9400000) {   /* LDR W */
                imm = ((i2 >> 10) & 0xfff) * 4; hit = 1;
            } else if ((i2 & 0xFFC00000) == 0x39400000) {   /* LDRB */
                imm = ((i2 >> 10) & 0xfff); hit = 1;
            } else if ((i2 & 0xFFC00000) == 0x79400000) {   /* LDRH */
                imm = ((i2 >> 10) & 0xfff) * 2; hit = 1;
            }
            if (hit && pg + imm == target) { pcs[n++] = TEXT_VA_ + off; break; }
            if (hit) break;
        }
    }
    return n;
}

/* ------------------------------------------------------------- poolmember
 * FNamePool+off reader with the "Reset" shape: subobject-guard + >=2 BLR +
 * >=2 direct BL calls (the Win64 ground truth 0x864786C shape).            */
static uint64_t poolmember_find(uint64_t off, char *how, size_t howcap)
{
    NamePoolRes np;
    if (!namepool_find(&np) || !np.pool) return 0;
    uint64_t target = np.pool + off;
    static uint64_t pcs[256];
    int n = dref_scan(target, pcs, 256);
    if (!n) return 0;
    uint64_t best = 0; int bestscore = -1;
    for (int i = 0; i < n; i++) {
        uint64_t fn = func_of(pcs[i]);
        if (!fn) continue;
        uint64_t e = func_end_of(fn);
        if (!e || e <= fn) continue;
        int nbl = 0, nblr = 0;
        for (uint64_t a = fn; a + 4 <= e; a += 4) {
            uint32_t w = insn_at(a);
            if (is_blr(w)) nblr++;
            else { uint64_t t = dec_bl(w, a); if (t && in_text(t)) nbl++; }
        }
        int insns = (int)((e - fn) / 4);
        int score = nblr * 2 + nbl - (insns > 96 ? 3 : 0);
        if (nblr >= 2 && nbl >= 2 && score > bestscore) {
            bestscore = score; best = fn;
            snprintf(how, howcap,
                "poolmember +0x%llX reader (fn 0x%llX: %d BLR, %d BL, %d insns; %d sites)",
                (unsigned long long)off, (unsigned long long)fn,
                nblr, nbl, insns, n);
        }
    }
    return best;
}
/* ========================================================================== */
/* List parsing, batch resolution, --all enumeration, offsets.h emission     */
/* ========================================================================== */

typedef struct {
    char name[96];
    uint64_t va;
    uint64_t thunk, orig;      /* Path A extras */
    uint64_t vt_base;          /* vtable base (vft / vtbase) */
    int      vt_index;         /* slot index (vft) */
    uint64_t vt_off;           /* byte offset */
    uint64_t va2;              /* secondary (give_ab, gis_server...) */
    int      kind;             /* 0 miss 1 func 2 var 3 site 4 vft */
    char how[160];
    char macro[128];
} Result;

static void sanitize_macro(const char *in, char *out, size_t cap){
    size_t j=0;
    for (size_t i=0; in[i] && j<cap-1; i++){
        char c=in[i];
        if (isalnum((unsigned char)c)) out[j++]=c;
        else if (c=='_') out[j++]='_';
        else out[j++]='_';
    }
    out[j]=0;
}

/* ---- single-mode resolution. returns 1 on hit. fills res ---- */
static int run_mode(const char *mode, Result *r, int verbose);

/* try each '|' mode in order */
static int run_entry_modes(char *modes, Result *r, int verbose){
    char *save=NULL;
    char *tok = strtok_r(modes, "|", &save);
    while (tok){
        while (*tok==' ') tok++;
        char *e=tok+strlen(tok);
        while (e>tok && (e[-1]==' '||e[-1]=='\t'||e[-1]=='\r'||e[-1]=='\n')) *--e=0;
        if (run_mode(tok, r, verbose)) return 1;
        tok = strtok_r(NULL, "|", &save);
    }
    return 0;
}

/* resolve Path B with W16+ASCII attempts, choose best cand */
static uint64_t path_b_any(const char *s, PathBCand *best_c, int *nc_out, int prefer_big)
{
    static PathBCand cands[96];
    int nc = path_b(s, 1, cands, 96, NULL);
    if (!nc) nc = path_b(s, 0, cands, 96, NULL);
    if (!nc) return 0;
    int best=0;
    for (int i=1;i<nc;i++){
        if (prefer_big){
            uint64_t si=func_end_of(cands[i].fn)-cands[i].fn, sb=func_end_of(cands[best].fn)-cands[best].fn;
            if (si>sb){best=i;continue;}
            if (si==sb && cands[i].votes>cands[best].votes){best=i;continue;}
        } else {
            if (cands[i].votes>cands[best].votes){best=i;continue;}
            if (cands[i].votes==cands[best].votes){
                int di=(cands[i].direct>0), db=(cands[best].direct>0);
                if (di!=db){ if(di){best=i;continue;} }
                uint64_t si=func_end_of(cands[i].fn)-cands[i].fn, sb=func_end_of(cands[best].fn)-cands[best].fn;
                if (si>sb){best=i;continue;}
            }
        }
    }
    if (best_c) *best_c=cands[best];
    if (nc_out) *nc_out=nc;
    return cands[best].fn;
}

/* vtable info for a function */
static int vft_info(uint64_t fn, uint64_t *base, int *index, uint64_t *off){
    uint64_t lo,hi;
    if (!rsa_find(fn,&lo,&hi)) return 0;
    for (uint64_t k=lo;k<hi;k++){
        uint64_t slot=RSA_[k].off;
        uint64_t b=vt_base_of_slot(slot);
        if (!b) continue;
        *base=b;
        *index=(int)((slot-b)/8);
        *off=slot-b;
        return 1;
    }
    return 0;
}

static int run_mode(const char *mode, Result *r, int verbose)
{
    (void)verbose;
    if (!strncmp(mode,"w:",2)){
        PathBCand cb; int nc;
        uint64_t f = 0;
        { static PathBCand cands[96]; nc = path_b(mode+2, 1, cands, 96, NULL);
          if (!nc) return 0;
          int best=0; for(int i=1;i<nc;i++) if (cands[i].votes>cands[best].votes) best=i;
          cb=cands[best]; f=cb.fn; }
        if (!f) return 0;
        r->va=f; r->kind=1;
        snprintf(r->how,sizeof(r->how),"w:\"%.60s\"",mode+2);
        return 1;
    }
    if (!strncmp(mode,"a:",2)){
        PathBCand cands[96]; int nc = path_b(mode+2, 0, cands, 96, NULL);
        if (!nc) return 0;
        int best=0; for(int i=1;i<nc;i++) if (cands[i].votes>cands[best].votes) best=i;
        r->va=cands[best].fn; r->kind=1;
        snprintf(r->how,sizeof(r->how),"a:\"%.60s\"",mode+2);
        return 1;
    }
    if (!strncmp(mode,"s:",2)){
        uint64_t f=path_b_any(mode+2,NULL,NULL,1);
        if (!f) return 0;
        r->va=f; r->kind=1;
        snprintf(r->how,sizeof(r->how),"s!:\"%.60s\"",mode+2);
        return 1;
    }
    if (!strncmp(mode,"n:",2)){
        PathARes pa[8]; int n=path_a(mode+2, pa, 8);
        if (!n) return 0;
        r->va=pa[0].thunk; r->thunk=pa[0].thunk; r->orig=pa[0].orig; r->kind=1;
        snprintf(r->how,sizeof(r->how),"native:\"%s\"%s",mode+2,
                 pa[0].orig?"":" (impl indirect - hook THUNK)");
        return 1;
    }
    if (!strncmp(mode,"vft:",4) || !strncmp(mode,"vfta:",5)){
        const char *inner = mode[3]==':' ? mode+4 : mode+5;
        uint64_t f;
        if (mode[3]=='a'){ PathARes pa[8]; int n=path_a(inner,pa,8); if(!n) return 0; f=pa[0].orig?pa[0].orig:pa[0].thunk; }
        else { f=path_b_any(inner,NULL,NULL,0); }
        if (!f) return 0;
        uint64_t base,off; int idx;
        if (vft_info(f,&base,&idx,&off)){
            r->va=f; r->vt_base=base; r->vt_index=idx; r->vt_off=off; r->kind=4;
            snprintf(r->how,sizeof(r->how),"vft slot %d (+0x%llX) base 0x%llX",
                idx,(unsigned long long)off,(unsigned long long)base);
            return 1;
        }
        r->va=f; r->kind=1;
        snprintf(r->how,sizeof(r->how),"func (no vtable slot)");
        return 1;
    }
    if (!strncmp(mode,"vftn:",5)){
        uint64_t f=path_b_any(mode+5,NULL,NULL,0);
        if (!f) return 0;
        uint64_t base,off; int idx;
        if (!vft_info(f,&base,&idx,&off)) return 0;
        uint64_t next_fn; uint64_t slot=base+off+8;
        if (rs_val(slot,&next_fn) && in_text(next_fn) && is_func_start(next_fn)){
            r->va=next_fn; r->vt_base=base; r->vt_index=idx+1; r->vt_off=off+8; r->kind=4;
            snprintf(r->how,sizeof(r->how),"vft slot+1 (%d) base 0x%llX",idx+1,(unsigned long long)base);
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"chain:",6)){
        uint64_t donor=path_b_any(mode+6,NULL,NULL,0);
        if (!donor) return 0;
        uint64_t f=0,g=0;
        if (chain_scan(donor,&f,&g) && f){
            r->va=f; r->va2=g; r->kind=1;
            snprintf(r->how,sizeof(r->how),"chain in donor 0x%llX (+B 0x%llX)",
                     (unsigned long long)donor,(unsigned long long)g);
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"cksz:",5) || !strncmp(mode,"ckszg:",6)){
        int want_give = (mode[4]=='g');
        const char *anch = mode + (want_give?6:5);
        for (int wide=1; wide>=0; wide--){
            uint64_t give=0;
            uint64_t c=cksz_find(anch,wide,&give);
            if (c){
                r->va = want_give ? give : c;
                r->va2 = want_give ? c : give;
                r->kind=1;
                snprintf(r->how,sizeof(r->how),"cksz:\"%s\" histogram (%s 0x%llX, %s 0x%llX)",anch,
                    want_give?"GiveAbility":"ConstructAbilitySpec",
                    (unsigned long long)(want_give?give:c),
                    want_give?"ConstructAbilitySpec":"GiveAbility",
                    (unsigned long long)(want_give?c:give));
                return 1; }
        }
        return 0;
    }
    if (!strncmp(mode,"gis:",4)){
        GisRes g;
        int n=gis_find(mode+4,&g);
        if (!n) return 0;
        r->kind=2;
        if (strstr(r->name,"Server")){ r->va=g.gis_server; r->va2=g.gis_client; }
        else { r->va=g.gis_client; r->va2=g.gis_server; }
        snprintf(r->how,sizeof(r->how),"GIsClient var + GIsServer var, via fn 0x%llX", (unsigned long long)g.fn);
        return 1;
    }
    if (!strncmp(mode,"namepool",8)){
        NamePoolRes np;
        if (!namepool_find(&np)) return 0;
        r->kind=2; r->va=np.pool; r->va2=np.ctor;
        snprintf(r->how,sizeof(r->how),"pool var; ctor 0x%llX; accessor 0x%llX; errFn 0x%llX",
            (unsigned long long)np.ctor,(unsigned long long)np.accessor,(unsigned long long)np.err_fn);
        return 1;
    }
    if (!strncmp(mode,"vtbase:",7)){
        VtBaseRes v;
        if (!vtbase_find(mode+7,&v)) return 0;
        r->kind=4; r->va=v.isnetready_fn; r->vt_base=v.vt_base; r->va2=v.tick_fn;
        r->vt_off=0x330; r->vt_index=0x330/8;
        snprintf(r->how,sizeof(r->how),"Tick 0x%llX; VTBase 0x%llX; IsNetReady@+0x330",
            (unsigned long long)v.tick_fn,(unsigned long long)v.vt_base);
        return 1;
    }
    if (!strncmp(mode,"encp:",5)){
        uint64_t s=encpatch_find(mode+5);
        if (!s) return 0;
        r->kind=3; r->va=s;
        snprintf(r->how,sizeof(r->how),"patch site (cond branch)");
        return 1;
    }
    if (!strncmp(mode,"anchor:",7)){
        uint64_t va=strtoull(mode+7,NULL,16);
        if (va && is_func_start(va)){
            r->kind=1; r->va=va;
            snprintf(r->how,sizeof(r->how),"documented anchor (validated)");
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"calleron2:",10)){
        /* common caller of RPC send-stubs of two natives (A~B) */
        const char *p=strchr(mode+10,'~');
        if (!p) return 0;
        char n1[96],n2[96];
        snprintf(n1,sizeof(n1),"%.*s",(int)(p-(mode+10)),mode+10);
        snprintf(n2,sizeof(n2),"%s",p+1);
        uint64_t s1=path_b_any(n1,NULL,NULL,0), s2=path_b_any(n2,NULL,NULL,0);
        if (!s1||!s2) return 0;
        CallSite *c1; uint64_t m1=callers_of(s1,&c1);
        CallSite *c2; uint64_t m2=callers_of(s2,&c2);
        uint64_t best=0; int bestv=0;
        for (uint64_t i=0;i<m1;i++){
            uint64_t f1=func_of(c1[i].pc); if(!f1) continue;
            for (uint64_t j=0;j<m2;j++){
                uint64_t f2=func_of(c2[j].pc); if(!f2) continue;
                if (f1==f2){ best=f1; bestv++; }
            }
        }
        if (best && bestv>0){
            r->kind=1; r->va=best;
            snprintf(r->how,sizeof(r->how),"common caller of %s+%s stubs",n1,n2);
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"blnext:",7)){
        /* resolve fn via anchor; first BL after the log-callsite inside it */
        uint64_t f=path_b_any(mode+7,NULL,NULL,0);
        if (!f) return 0;
        uint64_t e=func_end_of(f);
        if (!e||e<=f) return 0;
        StrHit *hits; uint64_t nh;
        for (int wide=1; wide>=0; wide--){
            if (!str_find_all(mode+7, wide, 1, &hits,&nh) || !nh){ str_free(hits); continue; }
            for (uint64_t h=0;h<nh;h++){
                RefSite *rs; uint64_t n=refs_to(hits[h].lit,&rs);
                for (uint64_t k=0;k<n;k++){
                    uint64_t pc=rs[k].pc;
                    uint64_t piece=func_of(pc);
                    /* hop up through outlined pieces until a call site inside f */
                    uint64_t site=0;
                    if (piece==f) site=pc;
                    else {
                        for (int hop=0; hop<12 && !site; hop++){
                            CallSite *cs; uint64_t m=callers_of(piece,&cs);
                            uint64_t nextp=0;
                            for (uint64_t q=0;q<m;q++){
                                uint64_t cf=func_of(cs[q].pc);
                                if (cf==f){ site=cs[q].pc; break; }
                                int kk=fkind_of(cf);
                                if ((kk==FK_BRIDGE||kk==FK_LOGTHUNK||kk==FK_TINY) && !nextp) nextp=cf;
                            }
                            if (!site && !nextp) break;
                            if (!site) piece=nextp;
                        }
                    }
                    if (!site) continue;
                    for (uint64_t a=site+4; a+4<=e; a+=4){
                        uint64_t t=dec_bl(insn_at(a),a);
                        if (t && in_text(t) && is_func_start(t) && heat_of(t)<50){
                            r->kind=1; r->va=t;
                            snprintf(r->how,sizeof(r->how),"BL after log site in 0x%llX",
                                (unsigned long long)f);
                            str_free(hits);
                            return 1;
                        }
                        if (a > site+0x100) break;
                    }
                }
            }
            str_free(hits);
        }
        return 0;
    }
    if (!strncmp(mode,"refs2:",6)){
        /* function referencing BOTH string literals A~B */
        char buf[300]; snprintf(buf,sizeof(buf),"%s",mode+6);
        char *p=strchr(buf,'~'); if(!p) return 0; *p=0;
        char *a=buf, *b=p+1;
        PathBCand ca[64],cb[64];
        int na=path_b(a,1,ca,64,NULL); if(!na) na=path_b(a,0,ca,64,NULL);
        int nb=path_b(b,1,cb,64,NULL); if(!nb) nb=path_b(b,0,cb,64,NULL);
        if (!na||!nb) return 0;
        uint64_t best=0; int bestv=0, nmatch=0;
        for (int i=0;i<na;i++) for (int j=0;j<nb;j++)
            if (ca[i].fn==cb[j].fn){
                nmatch++;
                int v=ca[i].votes+cb[j].votes;
                if (v>bestv){bestv=v;best=ca[i].fn;}
            }
        if (best){
            r->kind=1; r->va=best;
            snprintf(r->how,sizeof(r->how),"refs both \"%s\"+\"%s\"",a,b);
            { uint64_t base,off; int idx;
              if (vft_info(best,&base,&idx,&off))
                snprintf(r->how+strlen(r->how),sizeof(r->how)-strlen(r->how),
                    " (vft slot %d base 0x%llX)",idx,(unsigned long long)base); }
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"blbefore:",9)){
        /* the low-heat BL immediately before a call to F(resolved by anchor) */
        uint64_t f=path_b_any(mode+9,NULL,NULL,0);
        if (!f) return 0;
        CallSite *cs; uint64_t n=callers_of(f,&cs);
        uint64_t best=0; int bestd=0x1000;
        for (uint64_t k=0;k<n && k<4096;k++){
            uint64_t c=cs[k].pc;
            for (int j=1;j<=12;j++){
                uint64_t a=c-j*4;
                uint64_t t=dec_bl(insn_at(a),a);
                if (!t||!in_text(t)||!is_func_start(t)||t==f) continue;
                if (heat_of(t)>60) continue;
                uint64_t cf=func_of(c);
                if (func_of(a)!=cf) continue;
                if (j<bestd){ bestd=j; best=t; }
                break;
            }
        }
        if (best){ r->kind=1; r->va=best;
            snprintf(r->how,sizeof(r->how),"BL before call to 0x%llX",(unsigned long long)f);
            return 1; }
        return 0;
    }
    if (!strncmp(mode,"callerof:",9)){
        /* dominant BL-caller of the function resolved by anchor */
        uint64_t f=path_b_any(mode+9,NULL,NULL,0);
        if (!f) return 0;
        CallSite *cs; uint64_t n=callers_of(f,&cs);
        if (!n) return 0;
        uint64_t best=0; int bestv=0;
        for (uint64_t i=0;i<n && i<4096;i++){
            uint64_t g=func_of(cs[i].pc); if(!g||g==f) continue;
            int v=0;
            for (uint64_t j=0;j<n && j<4096;j++) if (func_of(cs[j].pc)==g) v++;
            if (v>bestv){bestv=v;best=g;}
        }
        if (best){ r->kind=1; r->va=best;
            snprintf(r->how,sizeof(r->how),"caller of 0x%llX",(unsigned long long)f);
            return 1; }
        return 0;
    }
    if (!strncmp(mode,"calleron:",9)){
        uint64_t stub=path_b_any(mode+9,NULL,NULL,0);
        if (!stub) return 0;
        CallSite *cs; uint64_t n=callers_of(stub,&cs);
        if (!n) return 0;
        /* dominant caller */
        uint64_t best=0; int bestv=0;
        for (uint64_t i=0;i<n && i<4096;i++){
            uint64_t f=func_of(cs[i].pc); if(!f||f==stub) continue;
            int v=0;
            for (uint64_t j=0;j<n && j<4096;j++) if (func_of(cs[j].pc)==f) v++;
            if (v>bestv){bestv=v;best=f;}
        }
        if (best){
            r->kind=1; r->va=best;
            snprintf(r->how,sizeof(r->how),"caller of %s stub",mode+9);
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"callee:",7)){
        /* callee:ANCHOR~N~shape  — Nth BL callee of resolved fn (1-based) */
        char buf[256]; snprintf(buf,sizeof(buf),"%s",mode+7);
        char *p1=strchr(buf,'~'); if(!p1) return 0; *p1=0;
        char *nstr=p1+1;
        char *p2=strchr(nstr,'~'); int shape=0;
        if (p2){ *p2=0; shape=1; }
        int nth=atoi(nstr); if(nth<1) nth=1;
        uint64_t f=path_b_any(buf,NULL,NULL,0);
        if (!f) return 0;
        uint64_t e=func_end_of(f); if(!e||e<=f) return 0;
        /* BL callees in order, dedup, func-starts */
        uint64_t seen[512]; int ns=0;
        for (uint64_t a=f;a+4<=e && ns<512;a+=4){
            uint64_t t=dec_bl(insn_at(a),a);
            if (!t||!in_text(t)||!is_func_start(t)) continue;
            int dup=0; for(int q=0;q<ns;q++) if(seen[q]==t){dup=1;break;}
            if (dup) continue;
            seen[ns++]=t;
        }
        for (int i=0;i<ns;i++){
            if (shape){
                int okb=0;
                /* CBZ/CBNZ X1 near start */
                for (int k=0;k<4;k++){
                    uint32_t w=insn_at(seen[i]+k*4);
                    if (is_cbzcbnz(w) && ((w>>5)&0x1f)==1) okb=1;
                }
                if (!okb) continue;
                int has_blr=0;
                uint64_t e2=func_end_of(seen[i]);
                if (!e2||e2<=seen[i]||e2-seen[i]>0x400) continue;
                for (uint64_t a=seen[i];a+4<=e2;a+=4) if (is_blr(insn_at(a))) {has_blr=1;break;}
                if (!has_blr) continue;
                r->kind=1; r->va=seen[i];
                snprintf(r->how,sizeof(r->how),"shaped callee of 0x%llX",(unsigned long long)f);
                return 1;
            }
            if (i==nth-1){
                r->kind=1; r->va=seen[i];
                snprintf(r->how,sizeof(r->how),"callee #%d of 0x%llX",nth,(unsigned long long)f);
                return 1;
            }
        }
        return 0;
    }
    if (!strncmp(mode,"execvslot:",10)){
        /* native name → exec thunk → inlined-wrapper vcall → vtable slot →
         * _Implementation (RemoveInventoryItem / …StateValue family)      */
        char buf[128]; snprintf(buf,sizeof(buf),"%s",mode+10);
        int nth = 0; char *t = strchr(buf,'~');
        if (t){ *t=0; nth=atoi(t+1); }   /* 0/absent = LAST vcall = _Impl */
        uint64_t fn,base,off; int idx; 
        if (execvslot_find(buf,nth,&fn,&base,&off,&idx,r->how,sizeof(r->how))){
            r->kind=4; r->va=fn; r->vt_base=base; r->vt_index=idx; r->vt_off=off;
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"blsite2:",8)){
        char buf[600]; snprintf(buf,sizeof(buf),"%s",mode+8);
        char *t = strchr(buf,'~');
        if (!t) return 0;
        *t = 0;
        uint64_t bl_pc=0,bl_t=0,enc=0;
        if (blsite2_find(buf,t+1,&bl_pc,&bl_t,&enc)){
            r->kind=3; r->va=bl_pc; r->va2=bl_t;
            snprintf(r->how,sizeof(r->how),
                "BL site 0x%llX in 0x%llX → Listen 0x%llX (verified: target calls CreateNetDriver)",
                (unsigned long long)bl_pc,(unsigned long long)enc,
                (unsigned long long)bl_t);
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"blsite:",7)){
        uint64_t bl_pc=0,bl_t=0,enc=0;
        if (blsite_find(mode+7,&bl_pc,&bl_t,&enc)){
            r->kind=3; r->va=bl_pc; r->va2=bl_t;
            snprintf(r->how,sizeof(r->how),
                "BL site 0x%llX in 0x%llX → target 0x%llX (hookable callsite)",
                (unsigned long long)bl_pc,(unsigned long long)enc,
                (unsigned long long)bl_t);
            return 1;
        }
        return 0;
    }
    if (!strncmp(mode,"poolmember:",11)){
        uint64_t off = strtoull(mode+11,NULL,0);
        if (off){
            uint64_t fn = poolmember_find(off,r->how,sizeof(r->how));
            if (fn){ r->kind=1; r->va=fn; return 1; }
        }
        return 0;
    }
    if (!strncmp(mode,"dref:",5)){
        uint64_t va = strtoull(mode+5,NULL,0);
        static uint64_t pcs[512];
        int n = va ? dref_scan(va,pcs,512) : 0;
        if (n){
            uint64_t fn = func_of(pcs[0]);
            if (fn){ r->kind=1; r->va=fn;
                snprintf(r->how,sizeof(r->how),"dref 0x%llX → fn 0x%llX (1st of %d refs)",
                    (unsigned long long)va,(unsigned long long)fn,n);
                return 1; }
            r->kind=1; r->va=pcs[0];
            snprintf(r->how,sizeof(r->how),"dref 0x%llX → site 0x%llX (%d refs, no fn)",
                (unsigned long long)va,(unsigned long long)pcs[0],n);
            return 1;
        }
        return 0;
    }
    if (mode[0]=='#'){
        /* documentation-only entry: records the reason for a known miss */
        snprintf(r->how,sizeof(r->how),"%s",mode+1);
        return 0;
    }
    /* default: Path B W16 then ASCII */
    {
        if (strlen(mode) < 3) return 0;
        uint64_t f=path_b_any(mode,NULL,NULL,0);
        if (!f) return 0;
        r->va=f; r->kind=1;
        snprintf(r->how,sizeof(r->how),"str:\"%.60s\"",mode);
        return 1;
    }
}

/* ------------------------------------------------------------- --all dump */
typedef struct {
    char name[256];
    uint64_t thunk, orig;
} NativeEntry;
static NativeEntry *NA_; static uint64_t NNA_, CAPNA_;

static int literal_name_at(uint64_t va, char *out, size_t cap, int *wide_out)
{
    const Elf64_Shdr *s=sec_of(va);
    if (!s || (s->sh_flags & SHF_EXEC_) || s->sh_type==SHT_NOBITS_) return 0;
    const uint8_t *p=va_ptr(va, 2);
    if (!p) return 0;
    /* try W16 */
    if (p[1]==0 && p[0]>=0x20 && p[0]<0x7f){
        char t[256]; uint64_t i=0;
        const uint8_t *q=p;
        for (;;){
            if (!va_ptr(va+i*2,2)) return 0;
            uint8_t lo=q[0], hi=q[1];
            if (lo==0 && hi==0) break;
            if (hi!=0 || lo<0x20 || lo>=0x7f) return 0;
            if (i>=250) return 0;
            t[i++]=(char)lo; q+=2;
        }
        if (i<2) return 0;
        t[i]=0;
        /* identifier-ish */
        for (uint64_t k=0;k<i;k++) if (!(isalnum((unsigned char)t[k])||t[k]=='_')) return 0;
        snprintf(out,cap,"%s",t);
        if (wide_out) *wide_out=1;
        return 1;
    }
    /* ASCII */
    if (p[0]>=0x20 && p[0]<0x7f){
        char t[256]; uint64_t i=0;
        const uint8_t *q=p;
        for (;;){
            const uint8_t *qq=va_ptr(va+i,1);
            if (!qq) return 0;
            uint8_t c=*qq;
            if (c==0) break;
            if (c<0x20||c>=0x7f) return 0;
            if (i>=250) return 0;
            t[i++]=(char)c;
        }
        if (i<2) return 0;
        t[i]=0;
        for (uint64_t k=0;k<i;k++) if (!(isalnum((unsigned char)t[k])||t[k]=='_')) return 0;
        snprintf(out,cap,"%s",t);
        if (wide_out) *wide_out=0;
        return 1;
    }
    return 0;
}
static int na_cmp(const void*a,const void*b){
    const NativeEntry *x=a,*y=b;
    int c=strcmp(x->name,y->name);
    if (c) return c;
    if (x->thunk<y->thunk) return -1; if (x->thunk>y->thunk) return 1; return 0;
}
static void run_all(void){
    for (uint64_t i=0;i<NRS_;i++){
        uint64_t slot=RS_[i].off, val=RS_[i].add;
        const Elf64_Shdr *ss=sec_of(slot);
        if (!ss || !sec_writable(ss)) continue;
        char nm[256]; int wide;
        if (!literal_name_at(val, nm, sizeof(nm), &wide)) continue;
        for (int buddy=8; buddy>=-8; buddy-=16){
            if (!buddy) continue;
            uint64_t tv;
            if (!rs_val(slot+buddy,&tv)) continue;
            if (!in_text(tv) || !is_func_start(tv)) continue;
            if (!is_exec_thunk(tv)) continue;
            /* dedup (name,thunk) */
            int dup=0;
            for (uint64_t q=0;q<NNA_;q++)
                if (NA_[q].thunk==tv && !strcmp(NA_[q].name,nm)){dup=1;break;}
            if (dup) continue;
            if (NNA_==CAPNA_){ CAPNA_=CAPNA_?CAPNA_*2:32768; NA_=xrealloc(NA_,CAPNA_*sizeof(NativeEntry)); }
            snprintf(NA_[NNA_].name,256,"%s",nm);
            NA_[NNA_].thunk=tv;
            NA_[NNA_].orig=thunk_orig(tv,NULL,100);
            NNA_++;
            break;
        }
    }
    qsort(NA_,NNA_,sizeof(NativeEntry),na_cmp);
}

/* ---------------------------------------------------------------- batch */
static Result *RES_; static int NRES_;

static void run_list(const char *path, int verbose)
{
    FILE *f=fopen(path,"r");
    if (!f){ printf("cannot open list %s\n",path); return; }
    char line[1024];
    RES_=xmalloc(sizeof(Result)*1024); NRES_=0;
    int idx=0;
    while (fgets(line,sizeof(line),f)){
        size_t l=strlen(line);
        while (l && (line[l-1]=='\n'||line[l-1]=='\r')) line[--l]=0;
        if (!l||line[0]=='#') continue;
        char *bar=strchr(line,'|');
        if (!bar) continue;
        *bar=0;
        char name[96]; snprintf(name,sizeof(name),"%s",line);
        /* trim */
        for (char *p=name;*p;p++) if(*p==' '&&*(p+1)==0)*p=0;
        char modes[900]; snprintf(modes,sizeof(modes),"%s",bar+1);
        Result *r=&RES_[NRES_]; memset(r,0,sizeof(*r));
        snprintf(r->name,sizeof(r->name),"%s",name);
        char mc[900]; snprintf(mc,sizeof(mc),"%s",modes);
        run_entry_modes(mc,r,verbose);
        NRES_++;
        idx++;
    }
    fclose(f);
    /* ---- execvslot majority re-vote: sibling RPCs of one class must share
     * the same owner vtable; adopt the majority base across entries and
     * re-resolve the minority at the majority base. */
    {
        uint64_t bases[32]; int votes[32]; int nb = 0;
        for (int i = 0; i < NRES_; i++) {
            if (RES_[i].kind != 4 || !RES_[i].vt_base || !RES_[i].vt_off) continue;
            if (strncmp(RES_[i].how, "execvslot:", 10) != 0) continue;
            int f = 0;
            for (int q = 0; q < nb; q++) if (bases[q] == RES_[i].vt_base) { votes[q]++; f = 1; break; }
            if (!f && nb < 32) { bases[nb] = RES_[i].vt_base; votes[nb] = 1; nb++; }
        }
        if (nb > 1) {
            int mbest = 0;
            for (int q = 1; q < nb; q++) if (votes[q] > votes[mbest]) mbest = q;
            if (votes[mbest] >= 2) {
                uint64_t mbase = bases[mbest];
                for (int i = 0; i < NRES_; i++) {
                    if (RES_[i].kind != 4 || RES_[i].vt_base != mbase) continue;
                    if (strncmp(RES_[i].how, "execvslot:", 10) != 0) continue;
                    /* nothing — already at majority base */
                }
                for (int i = 0; i < NRES_; i++) {
                    if (RES_[i].kind != 4 || !RES_[i].vt_off) continue;
                    if (strncmp(RES_[i].how, "execvslot:", 10) != 0) continue;
                    if (RES_[i].vt_base == mbase) continue;
                    uint64_t f2 = execvslot_at(mbase, RES_[i].vt_off);
                    if (f2) {
                        RES_[i].va = f2; RES_[i].vt_base = mbase;
                        RES_[i].vt_index = (int)(RES_[i].vt_off / 8);
                        size_t L = strlen(RES_[i].how);
                        snprintf(RES_[i].how + L, sizeof(RES_[i].how) - L,
                                 " [re-voted to majority base 0x%llX]",
                                 (unsigned long long)mbase);
                    }
                }
            }
        }
    }
    /* report */
    int hits=0;
    printf("%-38s %-10s %-12s %s\n","FINDER","STATUS","RVA","DETAILS");
    printf("--------------------------------------------------------------------------------------\n");
    for (int i=0;i<NRES_;i++){
        Result *r=&RES_[i];
        char va[32]="-";
        if (r->kind==2) snprintf(va,sizeof(va),"0x%llX",(unsigned long long)r->va);
        else if (r->kind) snprintf(va,sizeof(va),"0x%llX",(unsigned long long)r->va);
        if (r->kind) hits++;
        printf("%-38s %-10s %-12s %s\n", r->name,
               r->kind==0?"MISS":(r->kind==2?"VAR":(r->kind==3?"SITE":(r->kind==4?"VFT":"FUNC"))),
               va, r->how);
    }
    printf("--------------------------------------------------------------------------------------\n");
    printf("TOTAL: %d/%d found (%.1f%%)\n",hits,NRES_,NRES_?100.0*hits/NRES_:0);
}

/* ---------------------------------------------------------------- header */
static void emit_header(const char *path, const char *binname)
{
    FILE *f=fopen(path,"w");
    if (!f){ printf("cannot write %s\n",path); return; }
    char guard[128];
    { char b[96]; snprintf(b,sizeof(b),"%s_OFFSETS_H",binname);
      sanitize_macro(b,guard,sizeof(guard));
      for (char *p=guard;*p;p++) *p=(char)toupper((unsigned char)*p); }
    fprintf(f,"/* Auto-generated by uefinder v%s — do not edit */\n",VER);
    fprintf(f,"/* binary: %s */\n",binname);
    fprintf(f,"/* runtime = module_base + RVA */\n\n");
    fprintf(f,"#pragma once\n#ifndef %s\n#define %s\n\n",guard,guard);
    if (NRES_){
        fprintf(f,"/* ===== IMPORTANT: Finder.cpp tracked targets ===== */\n\n");
        for (int i=0;i<NRES_;i++){
            Result *r=&RES_[i];
            char m[192];
            char base[160]; sanitize_macro(r->name,base,sizeof(base));
            if (!r->kind){
                if (r->how[0])
                    fprintf(f,"/* %-40s NOT FOUND — %s */\n",r->name,r->how);
                else
                    fprintf(f,"/* %-40s NOT FOUND */\n",r->name);
                continue;
            }
            if (r->kind==2){
                snprintf(m,sizeof(m),"%s_RVA",base);
                fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->va);
                if (r->va2){
                    if (!strcmp(r->name,"FindGIsClient")||!strcmp(r->name,"FindGIsServer")){
                    } else if (!strncmp(r->name,"GIs",3)||strstr(r->name,"GIs")){
                    }
                    if (r->name[0]=='G'){
                        snprintf(m,sizeof(m),"%s_RVA",base);
                        /* gis: va2 = second variable */
                    }
                }
                if (!strcmp(r->name,"FindGIsClient")){
                    fprintf(f,"#define %-58s 0x%llX\n","GIsClient_RVA",(unsigned long long)r->va);
                    fprintf(f,"#define %-58s 0x%llX\n","GIsServer_RVA",(unsigned long long)r->va2);
                    fprintf(f,"/* variables (not functions); located via \"%s\") */\n",r->how);
                } else if (!strcmp(r->name,"FindGIsServer")){
                    fprintf(f,"#define %-58s 0x%llX\n","GIsServer_RVA",(unsigned long long)r->va);
                } else {
                    snprintf(m,sizeof(m),"%s_RVA",base);
                    fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->va);
                    if (r->va2) fprintf(f,"/* also: ctor/init 0x%llX */\n",(unsigned long long)r->va2);
                }
                continue;
            }
            if (r->kind==4){
                snprintf(m,sizeof(m),"%s_RVA",base);
                fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->va);
                if (r->vt_base){
                    snprintf(m,sizeof(m),"%s_VTBASE_RVA",base);
                    fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->vt_base);
                }
                if (r->vt_index>=0){
                    snprintf(m,sizeof(m),"%s_VT_INDEX",base);
                    fprintf(f,"#define %-58s %d\n",m,r->vt_index);
                    if (r->vt_off){
                        snprintf(m,sizeof(m),"%s_VT_OFF",base);
                        fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->vt_off);
                    }
                }
                fprintf(f,"/* %s */\n",r->how);
                continue;
            }
            snprintf(m,sizeof(m),"%s_RVA",base);
            fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->va);
            if (r->thunk || r->orig){
                if (r->thunk){ snprintf(m,sizeof(m),"%s_THUNK_RVA",base);
                    fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->thunk); }
                if (r->orig){ snprintf(m,sizeof(m),"%s_ORIG_RVA",base);
                    fprintf(f,"#define %-58s 0x%llX\n",m,(unsigned long long)r->orig); }
            }
            if (r->kind==3) fprintf(f,"/* %s */\n",r->how);
            if (r->va2) fprintf(f,"/* secondary: 0x%llX */\n",(unsigned long long)r->va2);
        }
        fprintf(f,"\n");
    }
    if (NNA_){
        fprintf(f,"/* ===== ALL NATIVE UFunctions (exec thunks + impl) ===== */\n\n");
        int dup_idx=0; char prev[256]; prev[0]=0;
        for (uint64_t i=0;i<NNA_;i++){
            NativeEntry *e=&NA_[i];
            char base[300]; sanitize_macro(e->name,base,sizeof(base));
            char m[360];
            if (prev[0] && !strcmp(prev,e->name)) dup_idx++;
            else { dup_idx=1; snprintf(prev,sizeof(prev),"%s",e->name); }
            if (dup_idx==1) snprintf(m,sizeof(m),"UF_%s_THUNK_RVA",base);
            else snprintf(m,sizeof(m),"UF_%s_%d_THUNK_RVA",base,dup_idx);
            fprintf(f,"#define %-64s 0x%llX\n",m,(unsigned long long)e->thunk);
            if (e->orig){
                if (dup_idx==1) snprintf(m,sizeof(m),"UF_%s_ORIG_RVA",base);
                else snprintf(m,sizeof(m),"UF_%s_%d_ORIG_RVA",base,dup_idx);
                fprintf(f,"#define %-64s 0x%llX\n",m,(unsigned long long)e->orig);
            }
        }
    }
    fprintf(f,"\n#endif /* %s */\n",guard);
    fclose(f);
    printf("header written: %s (%d important, %llu natives)\n",
        path,NRES_,(unsigned long long)NNA_);
}

/* ------------------------------------------------------------ dump mode */
static void dump_all(const char *prefix)
{
    char p[512]; FILE *f;
    /* functions */
    snprintf(p,sizeof(p),"%s.funcs",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(FS_,8,NFS_,f); fclose(f); }
    /* function kinds */
    snprintf(p,sizeof(p),"%s.fkinds",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(FK_,sizeof(int),NFS_,f); fclose(f); }
    /* ADRP+ADD refs: (tgt,pc) pairs */
    snprintf(p,sizeof(p),"%s.refs",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(REF_,sizeof(RefSite),NREF_,f); fclose(f); }
    /* ADRP+LDR refs */
    snprintf(p,sizeof(p),"%s.refl",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(REFL_,sizeof(RefSite),NREFL_,f); fclose(f); }
    /* BL sites (pc,tgt) */
    snprintf(p,sizeof(p),"%s.bl",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(BL_,sizeof(CallSite),NBL_,f); fclose(f); }
    /* relocation slots (off,add) */
    snprintf(p,sizeof(p),"%s.rels",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(RS_,sizeof(RelSlot),NRS_,f); fclose(f); }
    /* vtable slots + runs */
    snprintf(p,sizeof(p),"%s.vtslots",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(VT_SLOT_,8,VT_N_,f); fclose(f); }
    snprintf(p,sizeof(p),"%s.vtruns",prefix);
    f=fopen(p,"wb");
    if(f){ fwrite(VT_RUN_,8,VT_NR_,f); fclose(f); }
    printf("dumped: %llu funcs, %llu refs, %llu refl, %llu bl, %llu rels, %llu vtslots\n",
        (unsigned long long)NFS_,(unsigned long long)NREF_,(unsigned long long)NREFL_,
        (unsigned long long)NBL_,(unsigned long long)NRS_,(unsigned long long)VT_N_);
}

/* ------------------------------------------------------------------ diag */
static void diag(void){
    printf("== uefinder v%s diag ==\n",VER);
    printf("sections: %d  phdrs: %d\n",NSH_,NPH_);
    if (I_TEXT>=0) printf(".text va=0x%llX size=0x%llX\n",(unsigned long long)SH_[I_TEXT].sh_addr,(unsigned long long)SH_[I_TEXT].sh_size);
    if (I_RODATA>=0) printf(".rodata va=0x%llX size=0x%llX\n",(unsigned long long)SH_[I_RODATA].sh_addr,(unsigned long long)SH_[I_RODATA].sh_size);
    if (I_DRELRO>=0) printf(".data.rel.ro va=0x%llX size=0x%llX\n",(unsigned long long)SH_[I_DRELRO].sh_addr,(unsigned long long)SH_[I_DRELRO].sh_size);
    printf("dyn: rela=0x%llX/%llX arela=0x%llX/%llX relr=0x%llX/%llX\n",
        (unsigned long long)DY_.rela,(unsigned long long)DY_.relasz,
        (unsigned long long)DY_.arela,(unsigned long long)DY_.arelasz,
        (unsigned long long)DY_.relr,(unsigned long long)DY_.relrsz);
    printf(".eh_frame_hdr idx=%d  .eh_frame idx=%d\n", I_EHFRAMEHDR, I_EHFRAME);
    printf("reloc slots (RELATIVE): %llu\n",(unsigned long long)NRS_);
    printf("functions: %llu\n",(unsigned long long)NFS_);
    printf("ADRP+ADD refs: %llu   ADRP+LDR refs: %llu\n",(unsigned long long)NREF_,(unsigned long long)NREFL_);
    printf("BL/B sites: %llu\n",(unsigned long long)NBL_);
    printf("vtable slots: %llu\n",(unsigned long long)VT_N_);
    int nb=0,nl=0,nt=0;
    for (uint64_t i=0;i<NFS_;i++){ if(FK_[i]==FK_BRIDGE)nb++; else if(FK_[i]==FK_LOGTHUNK)nl++; else if(FK_[i]==FK_TINY)nt++; }
    printf("kinds: normal=%llu bridge=%llu logthunk=%llu tiny=%llu\n",
        (unsigned long long)(NFS_-nb-nl-nt),(unsigned long long)nb,(unsigned long long)nl,(unsigned long long)nt);
}

/* ------------------------------------------------------------------ main */
static int load_elf(const char *path)
{
    FILE *f=fopen(path,"rb");
    if (!f){ printf("cannot open %s\n",path); return 0; }
    fseek(f,0,SEEK_END); long sz=ftell(f); fseek(f,0,SEEK_SET);
    D_=xmalloc((uint64_t)sz);
    if (fread(D_,1,(uint64_t)sz,f)!=(size_t)sz){ printf("read error\n"); fclose(f); return 0; }
    fclose(f);
    N_=(uint64_t)sz;
    if (N_<64 || memcmp(D_,"\x7F""ELF",4) || D_[4]!=2){ printf("not ELF64\n"); return 0; }
    EH_=(Elf64_Ehdr*)D_;
    if (!EH_->e_shoff || !EH_->e_shnum){ printf("no sections\n"); return 0; }
    NSH_=EH_->e_shnum<MAX_SECS?EH_->e_shnum:MAX_SECS;
    memcpy(SH_, D_+EH_->e_shoff, NSH_*sizeof(Elf64_Shdr));
    SHSTR_=(const char*)(D_+SH_[EH_->e_shstrndx].sh_offset);
    NPH_=EH_->e_phnum; PH_=(Elf64_Phdr*)(D_+EH_->e_phoff);
    for (int i=0;i<NSH_;i++){
        const char *n=sec_name(&SH_[i]);
        if (!strcmp(n,".text")) I_TEXT=i;
        else if (!strcmp(n,".rodata")) I_RODATA=i;
        else if (!strcmp(n,".data.rel.ro")) I_DRELRO=i;
        else if (!strcmp(n,".eh_frame")) I_EHFRAME=i;
        else if (!strcmp(n,".eh_frame_hdr")) I_EHFRAMEHDR=i;
        else if (!strcmp(n,".init_array")) I_INITARRAY=i;
    }
    if (I_TEXT<0){ printf("no .text\n"); return 0; }
    TEXT_VA_=SH_[I_TEXT].sh_addr; TEXT_SZ_=SH_[I_TEXT].sh_size; TEXT_P_=D_+SH_[I_TEXT].sh_offset;
    parse_dynamic();
    load_relocations();
    build_rsa();
    build_funcs();
    build_refs();
    build_calls();
    build_vtables();
    classify_all();
    return 1;
}

int main(int argc, char **argv)
{
    if (argc<2){
        printf("uefinder v%s — UE/Fortnite Android ARM64 static finder\n",VER);
        printf("usage: uefinder <libUnreal.so|libUE4.so> [options]\n");
        printf("  -l <list>       batch finder list (Name | mode | mode...)\n");
        printf("  --all           enumerate all native UFunctions\n");
        printf("  --header <f>    write offsets.h (curated first, then all)\n");
        printf("  -s <str>        find function containing string (W16+ASCII)\n");
        printf("  -n <name>       find native by name (Path A)\n");
        printf("  -v              verbose\n");
        printf("  --diag          binary diagnostics\n");
        return 0;
    }
    int verbose=0;
    const char *listp=NULL,*headerp=NULL,*sfind=NULL,*nfind=NULL,*dbgfn=NULL;
    int do_all=0,do_diag=0; const char *dumpp=NULL;
    for (int i=2;i<argc;i++){
        if (!strcmp(argv[i],"-l")&&i+1<argc) listp=argv[++i];
        else if (!strcmp(argv[i],"--all")) do_all=1;
        else if (!strcmp(argv[i],"--header")&&i+1<argc) headerp=argv[++i];
        else if (!strcmp(argv[i],"-s")&&i+1<argc) sfind=argv[++i];
        else if (!strcmp(argv[i],"-n")&&i+1<argc) nfind=argv[++i];
        else if (!strcmp(argv[i],"--fn")&&i+1<argc) dbgfn=argv[++i];
        else if (!strcmp(argv[i],"-v")) verbose=1;
        else if (!strcmp(argv[i],"--diag")) do_diag=1;
        else if (!strcmp(argv[i],"--dump")&&i+1<argc) dumpp=argv[++i];
    }
    if (dumpp){
        if (!load_elf(argv[1])) return 1;
        dump_all(dumpp);
        return 0;
    }
    if (!load_elf(argv[1])) return 1;
    { /* derive binary name for header guard */
        const char *b=strrchr(argv[1],'/'); b=b?b+1:argv[1];
        snprintf(SO_NAME,sizeof(SO_NAME),"%s",b);
    }
    if (do_diag) diag();
    if (sfind){
        if (verbose){
            for (int wide=1; wide>=0; wide--){
                StrHit *hits; uint64_t nh;
                if (!str_find_all(sfind, wide, 1, &hits,&nh) || !nh){ str_free(hits); continue; }
                printf("[enc=%d] %llu literals:\n",wide,(unsigned long long)nh);
                for (uint64_t h=0;h<nh;h++){
                    printf("  lit 0x%llX:",(unsigned long long)hits[h].lit);
                    RefSite *rs; uint64_t n=refs_to(hits[h].lit,&rs);
                    for (uint64_t r=0;r<n;r++)
                        printf(" ref@0x%llX(fn 0x%llX,k%d)",(unsigned long long)rs[r].pc,
                            (unsigned long long)func_of(rs[r].pc), fkind_of(func_of(rs[r].pc)));
                    printf("\n");
                }
                str_free(hits);
            }
        }
        PathBCand bc; uint64_t f=path_b_any(sfind,&bc,NULL,0);
        if (f){
            printf("string \"%s\" → 0x%llX  (votes %d, %d candidates)\n",sfind,(unsigned long long)f,bc.votes,1);
            uint64_t base,off; int idx;
            if (vft_info(f,&base,&idx,&off))
                printf("  vft: index %d (+0x%llX) base 0x%llX\n",idx,(unsigned long long)off,(unsigned long long)base);
        } else printf("string not found / no refs\n");
    }
    if (dbgfn){
        uint64_t f=strtoull(dbgfn,NULL,16);
        for (int pass=0; pass<2; pass++){
            if (!f) break;
            printf("== fn 0x%llX kind=%d extent=[0x%llX..0x%llX) prologue=%d ==\n",
                (unsigned long long)f, fkind_of(f), (unsigned long long)f,
                (unsigned long long)func_end_of(f), is_prologue_at(f));
            printf("BLs:\n");
            uint64_t e=func_end_of(f);
            for (uint64_t a=f; a+4<=e; a+=4){
                uint64_t t=dec_bl(insn_at(a),a);
                if (t) printf("  +0x%03llX BL 0x%llX (heat %llu, start=%d)\n",
                    (unsigned long long)(a-f),(unsigned long long)t,
                    (unsigned long long)heat_of(t), is_func_start(t));
                if (is_blr(insn_at(a))) printf("  +0x%03llX BLR X%d\n",(unsigned long long)(a-f),(int)((insn_at(a)>>5)&0x1f));
            }
            printf("Bs / B.cond:\n");
            for (uint64_t a=f; a+4<=e; a+=4){
                uint32_t w=insn_at(a);
                uint64_t t=dec_b(w,a);
                if (t) printf("  +0x%03llX B 0x%llX (in fn 0x%llX mid=%d)\n",
                    (unsigned long long)(a-f),(unsigned long long)t,
                    (unsigned long long)func_of(t), t!=func_of(t));
                else if (is_bcond(w)||is_cbzcbnz(w)||is_tbztbnz(w)){
                    /* decode bcond target */
                    int64_t imm = (w>>5)&0x7ffff; if (imm & 0x40000) imm |= ~0x7ffff;
                    uint64_t tt = a + ((uint64_t)imm<<2);
                    printf("  +0x%03llX cond→0x%llX\n",(unsigned long long)(a-f),(unsigned long long)tt);
                }
            }
            CallSite *cs; uint64_t n=callers_of(f,&cs);
            printf("callers: %llu\n",(unsigned long long)n);
            for (uint64_t i=0;i<n && i<12;i++)
                printf("  from 0x%llX (fn 0x%llX)\n",(unsigned long long)cs[i].pc,(unsigned long long)func_of(cs[i].pc));
            {   uint64_t b,off; int idx;
                if (vft_info(f,&b,&idx,&off)) printf("vft: base 0x%llX idx %d off 0x%llX\n",
                    (unsigned long long)b,idx,(unsigned long long)off); }
            /* follow first BL caller */
            if (pass==0 && n>=1){
                uint64_t nf=func_of(cs[0].pc);
                if (nf && nf!=f){ f=nf; continue; }
            }
            break;
        }
    }
    if (nfind){
        PathARes pa[8]; int n=path_a(nfind,pa,8);
        for (int i=0;i<n;i++)
            printf("native %s[%d]: thunk 0x%llX orig 0x%llX slot 0x%llX buddy %+d\n",
                nfind,i,(unsigned long long)pa[i].thunk,(unsigned long long)pa[i].orig,
                (unsigned long long)pa[i].slot,pa[i].buddy);
        if (!n) printf("native %s not found\n",nfind);
    }
    if (listp) run_list(listp,verbose);
    if (do_all) run_all();
    if (headerp) emit_header(headerp,SO_NAME);
    return 0;
}
