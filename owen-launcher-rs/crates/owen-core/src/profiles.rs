//! Per-version redirect profiles.
//!
//! Owen 2.x decides how to redirect each Fortnite version:
//!
//! * **Kairos era (10.31 - 15.50)** — the WebView frontend. Redirected by
//!   ART-hooking `DebouncedLimiter.loginUrl` / `getGraphqlUrl` /
//!   `registerUrl` / `getHostName` plus a `WebView.loadUrl` safety net.
//!   No smali is ever edited.
//! * **Native era (16.00+)** — HTTP flows through libcurl inside
//!   libUnreal.so and libEOSSDK.so. `curl_easy_setopt` is located by the
//!   automatic scanner ([`crate::scanner`]) and hooked; the legacy
//!   `FCurlHttpRequest::ProcessRequest` offsets below remain as a fallback
//!   for images the scanner cannot score.
//! * **libUE4.so era (<= 14.60 on some builds)** — `curl_easy_setopt` is
//!   exported, so it is resolved by symbol and hooked.

use crate::version::FortniteVersion;
use serde::{Deserialize, Serialize};

/// Which redirect mechanisms apply to a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedirectMethod {
    /// Kairos WebView era: ART hooks on DebouncedLimiter + WebView.loadUrl.
    KairosWeb,
    /// Native era: automatic curl_easy_setopt detection + hook.
    NativeCurl,
    /// libUE4.so with exported curl symbols.
    ExportedCurl,
    /// Unknown version: try everything.
    Auto,
}

/// FCurlHttpRequest offsets for one build.
#[derive(Debug, Clone, Serialize)]
pub struct FcurlOffsets {
    pub version: &'static str,
    pub pr_offset: u32,
    pub geturl_off: u32,
    pub seturl_off: u32,
}

/// FCurlHttpRequest::ProcessRequest offsets inside libUnreal.so,
/// (version, pr_offset, geturl_off, seturl_off). Legacy fallback path —
/// the primary redirect path is automatic curl_easy_setopt detection.
pub const UE_FCURL_OFFSETS: &[FcurlOffsets] = &[
    FcurlOffsets { version: "++Fortnite+Release-18.40-CL-18167774", pr_offset: 0x95FB1C4, geturl_off: 0x70, seturl_off: 0x95F8108 },
    FcurlOffsets { version: "++Fortnite+Release-19.00-CL-18335626", pr_offset: 0x9FC7F18, geturl_off: 0x70, seturl_off: 0x9FC5A20 },
    FcurlOffsets { version: "++Fortnite+Release-19.00-CL-18380290", pr_offset: 0x9FC8064, geturl_off: 0x70, seturl_off: 0x9FC5B6C },
    FcurlOffsets { version: "++Fortnite+Release-19.01-CL-18441387", pr_offset: 0xA04AEF4, geturl_off: 0x70, seturl_off: 0xA0489FC },
    FcurlOffsets { version: "++Fortnite+Release-19.01-CL-18467134", pr_offset: 0xA04B09C, geturl_off: 0x70, seturl_off: 0xA048BA4 },
    FcurlOffsets { version: "++Fortnite+Release-19.10-CL-18593273", pr_offset: 0xB98A7C8, geturl_off: 0x70, seturl_off: 0xB988370 },
    FcurlOffsets { version: "++Fortnite+Release-19.10-CL-18642366", pr_offset: 0xB98B468, geturl_off: 0x70, seturl_off: 0xB989010 },
    FcurlOffsets { version: "++Fortnite+Release-19.10-CL-18675304", pr_offset: 0xB98B690, geturl_off: 0x70, seturl_off: 0xB989238 },
    FcurlOffsets { version: "++Fortnite+Release-19.20-CL-18778007", pr_offset: 0xB972F00, geturl_off: 0x70, seturl_off: 0xB970A90 },
    FcurlOffsets { version: "++Fortnite+Release-19.30-CL-18931164", pr_offset: 0x818DAA8, geturl_off: 0x70, seturl_off: 0x818B650 },
    FcurlOffsets { version: "++Fortnite+Release-19.30-CL-19036451", pr_offset: 0x818E1C0, geturl_off: 0x70, seturl_off: 0x818BD68 },
    FcurlOffsets { version: "++Fortnite+Release-19.40-CL-19121574", pr_offset: 0xBD05208, geturl_off: 0x70, seturl_off: 0xBD02BC4 },
    FcurlOffsets { version: "++Fortnite+Release-20.00-CL-19411831", pr_offset: 0xC927874, geturl_off: 0x70, seturl_off: 0xC925220 },
    FcurlOffsets { version: "++Fortnite+Release-20.10-CL-19582505", pr_offset: 0x86D6468, geturl_off: 0x70, seturl_off: 0x86D400C },
    FcurlOffsets { version: "++Fortnite+Release-20.20-CL-19751212", pr_offset: 0x87ADAD8, geturl_off: 0x70, seturl_off: 0x87AB660 },
    FcurlOffsets { version: "++Fortnite+Release-20.20-CL-19864603", pr_offset: 0x87ADFE4, geturl_off: 0x70, seturl_off: 0x87ABB6C },
    FcurlOffsets { version: "++Fortnite+Release-20.30-CL-20005561", pr_offset: 0x87A901C, geturl_off: 0x70, seturl_off: 0x87A6BA8 },
    FcurlOffsets { version: "++Fortnite+Release-20.30-CL-20037261", pr_offset: 0x87A9204, geturl_off: 0x70, seturl_off: 0x87A6D90 },
    FcurlOffsets { version: "++Fortnite+Release-20.40-CL-20175294", pr_offset: 0x88C1340, geturl_off: 0x70, seturl_off: 0x88BEEC8 },
    FcurlOffsets { version: "++Fortnite+Release-20.40-CL-20244966", pr_offset: 0x88C178C, geturl_off: 0x70, seturl_off: 0x88BF314 },
    FcurlOffsets { version: "++Fortnite+Release-21.00-CL-20389800", pr_offset: 0x8A6A088, geturl_off: 0x70, seturl_off: 0x8A67C10 },
    FcurlOffsets { version: "++Fortnite+Release-21.00-CL-20580074", pr_offset: 0x8A6AE30, geturl_off: 0x70, seturl_off: 0x8A689B8 },
    FcurlOffsets { version: "++Fortnite+Release-21.10-CL-20696680", pr_offset: 0x8BDE4C4, geturl_off: 0x70, seturl_off: 0x8BDC0BC },
    FcurlOffsets { version: "++Fortnite+Release-21.10-CL-20805132", pr_offset: 0x8BDEAB4, geturl_off: 0x70, seturl_off: 0x8BDC6AC },
    FcurlOffsets { version: "++Fortnite+Release-21.20-CL-20938145", pr_offset: 0x8C12EF8, geturl_off: 0x70, seturl_off: 0x8C10AF0 },
    FcurlOffsets { version: "++Fortnite+Release-21.20-CL-20978394", pr_offset: 0x8C12B3C, geturl_off: 0x70, seturl_off: 0x8C10734 },
    FcurlOffsets { version: "++Fortnite+Release-21.20-CL-21014428", pr_offset: 0x8C13048, geturl_off: 0x70, seturl_off: 0x8C10C40 },
    FcurlOffsets { version: "++Fortnite+Release-21.30-CL-21088273", pr_offset: 0x89F02F0, geturl_off: 0x70, seturl_off: 0x89EDCF8 },
    FcurlOffsets { version: "++Fortnite+Release-21.40-CL-21347928", pr_offset: 0x8761B30, geturl_off: 0x70, seturl_off: 0x875F794 },
    FcurlOffsets { version: "++Fortnite+Release-21.40-CL-21431554", pr_offset: 0x8763A6C, geturl_off: 0x70, seturl_off: 0x87616D0 },
    FcurlOffsets { version: "++Fortnite+Release-21.50-CL-21557619", pr_offset: 0x878EEE4, geturl_off: 0x70, seturl_off: 0x878CEAC },
    FcurlOffsets { version: "++Fortnite+Release-21.51-CL-21785624", pr_offset: 0x87DA864, geturl_off: 0x70, seturl_off: 0x87D882C },
    FcurlOffsets { version: "++Fortnite+Release-22.00-CL-22031344", pr_offset: 0x8CD9F20, geturl_off: 0x70, seturl_off: 0x8CD83CC },
    FcurlOffsets { version: "++Fortnite+Release-22.00-CL-22107157", pr_offset: 0x8CE1170, geturl_off: 0x70, seturl_off: 0x8CDF61C },
    FcurlOffsets { version: "++Fortnite+Release-22.10-CL-22240570", pr_offset: 0x932D314, geturl_off: 0x70, seturl_off: 0x932B7C0 },
    FcurlOffsets { version: "++Fortnite+Release-22.20-CL-22545427", pr_offset: 0x932DE98, geturl_off: 0x70, seturl_off: 0x932C150 },
    FcurlOffsets { version: "++Fortnite+Release-22.30-CL-22854046", pr_offset: 0x8F2F4C4, geturl_off: 0x70, seturl_off: 0x8F2D77C },
    FcurlOffsets { version: "++Fortnite+Release-22.30-CL-22886174", pr_offset: 0x8F2F740, geturl_off: 0x70, seturl_off: 0x8F2D9F8 },
    FcurlOffsets { version: "++Fortnite+Release-22.40-CL-23070899", pr_offset: 0x8FE0F9C, geturl_off: 0x70, seturl_off: 0x8FDF254 },
    FcurlOffsets { version: "++Fortnite+Release-23.00-CL-23281232", pr_offset: 0x6AA9BD8, geturl_off: 0x70, seturl_off: 0x8FFF24C },
    FcurlOffsets { version: "++Fortnite+Release-23.00-CL-23385779", pr_offset: 0x6AAF768, geturl_off: 0x70, seturl_off: 0x8FFD0B8 },
    FcurlOffsets { version: "++Fortnite+Release-23.10-CL-23443094", pr_offset: 0x69AB370, geturl_off: 0x70, seturl_off: 0x915CF10 },
    FcurlOffsets { version: "++Fortnite+Release-23.10-CL-23572221", pr_offset: 0x69ACD44, geturl_off: 0x70, seturl_off: 0x915F73C },
    FcurlOffsets { version: "++Fortnite+Release-23.20-CL-23659353", pr_offset: 0x6F2B3FC, geturl_off: 0x70, seturl_off: 0x94EDFD8 },
    FcurlOffsets { version: "++Fortnite+Release-23.20-CL-23783097", pr_offset: 0x6FE5E88, geturl_off: 0x70, seturl_off: 0x95413E0 },
    FcurlOffsets { version: "++Fortnite+Release-23.30-CL-23901854", pr_offset: 0x702FBE4, geturl_off: 0x70, seturl_off: 0x95B15F8 },
    FcurlOffsets { version: "++Fortnite+Release-23.30-CL-23937783", pr_offset: 0x70303DC, geturl_off: 0x70, seturl_off: 0x95B24D8 },
    FcurlOffsets { version: "++Fortnite+Release-23.30-CL-23986860", pr_offset: 0x7030908, geturl_off: 0x70, seturl_off: 0x95B472C },
    FcurlOffsets { version: "++Fortnite+Release-23.40-CL-24087481", pr_offset: 0x6E135A8, geturl_off: 0x70, seturl_off: 0x9701750 },
    FcurlOffsets { version: "++Fortnite+Release-23.40-CL-24230880", pr_offset: 0x6E133D4, geturl_off: 0x70, seturl_off: 0x9701A98 },
    FcurlOffsets { version: "++Fortnite+Release-23.50-CL-24376996", pr_offset: 0x6EFF15C, geturl_off: 0x70, seturl_off: 0x97220D8 },
    FcurlOffsets { version: "++Fortnite+Release-24.00-CL-24518431", pr_offset: 0x7108914, geturl_off: 0x70, seturl_off: 0x7109570 },
    FcurlOffsets { version: "++Fortnite+Release-24.01-CL-24665656", pr_offset: 0x71799D8, geturl_off: 0x70, seturl_off: 0x717A634 },
    FcurlOffsets { version: "++Fortnite+Release-24.10-CL-24770548", pr_offset: 0x71C8AB4, geturl_off: 0x70, seturl_off: 0x71C9710 },
    FcurlOffsets { version: "++Fortnite+Release-24.20-CL-24939793", pr_offset: 0x72DA320, geturl_off: 0x70, seturl_off: 0x72DAFC0 },
    FcurlOffsets { version: "++Fortnite+Release-24.30-CL-25280217", pr_offset: 0x712C410, geturl_off: 0x70, seturl_off: 0x712D0B0 },
    FcurlOffsets { version: "++Fortnite+Release-24.30-CL-25330280", pr_offset: 0x712CA1C, geturl_off: 0x70, seturl_off: 0x712D6BC },
    FcurlOffsets { version: "++Fortnite+Release-24.40-CL-25469380", pr_offset: 0x700D6FC, geturl_off: 0x70, seturl_off: 0x700E39C },
    FcurlOffsets { version: "++Fortnite+Release-24.40-CL-25595478", pr_offset: 0x700E1FC, geturl_off: 0x70, seturl_off: 0x700EE9C },
    FcurlOffsets { version: "++Fortnite+Release-25.00-CL-25695576", pr_offset: 0x739A09C, geturl_off: 0x70, seturl_off: 0x739AD44 },
    FcurlOffsets { version: "++Fortnite+Release-25.00-CL-25913641", pr_offset: 0x739C67C, geturl_off: 0x70, seturl_off: 0x739D324 },
    FcurlOffsets { version: "++Fortnite+Release-25.10-CL-26000958", pr_offset: 0x73B68A4, geturl_off: 0x70, seturl_off: 0x73B754C },
    FcurlOffsets { version: "++Fortnite+Release-25.11-CL-26227037", pr_offset: 0x73B85F8, geturl_off: 0x70, seturl_off: 0x73B92A0 },
    FcurlOffsets { version: "++Fortnite+Release-25.20-CL-26474516", pr_offset: 0x76D98A8, geturl_off: 0x70, seturl_off: 0x76DA550 },
    FcurlOffsets { version: "++Fortnite+Release-25.20-CL-26643816", pr_offset: 0x76D6A70, geturl_off: 0x70, seturl_off: 0x76D7718 },
    FcurlOffsets { version: "++Fortnite+Release-25.30-CL-26867995", pr_offset: 0x77103EC, geturl_off: 0x70, seturl_off: 0x7711094 },
    FcurlOffsets { version: "++Fortnite+Release-26.00-CL-27233190", pr_offset: 0x80A49C4, geturl_off: 0x78, seturl_off: 0x7D7EE28 },
    FcurlOffsets { version: "++Fortnite+Release-26.00-CL-27405594", pr_offset: 0x80A54A0, geturl_off: 0x78, seturl_off: 0x7D7F480 },
    FcurlOffsets { version: "++Fortnite+Release-26.10-CL-27665530", pr_offset: 0x81CA8C0, geturl_off: 0x78, seturl_off: 0x7EA9220 },
    FcurlOffsets { version: "++Fortnite+Release-26.20-CL-28096793", pr_offset: 0x7ABD100, geturl_off: 0x78, seturl_off: 0x7ABE024 },
    FcurlOffsets { version: "++Fortnite+Release-26.20-CL-28333111", pr_offset: 0x7ABD108, geturl_off: 0x78, seturl_off: 0x7ABE02C },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28520140", pr_offset: 0x7B31F28, geturl_off: 0x78, seturl_off: 0x80F59A0 },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28620401", pr_offset: 0x7B31950, geturl_off: 0x78, seturl_off: 0x80F53D4 },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28688692", pr_offset: 0x7B34CB0, geturl_off: 0x78, seturl_off: 0x80F885C },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28849396", pr_offset: 0x7B35108, geturl_off: 0x78, seturl_off: 0x80F8CB4 },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28938851", pr_offset: 0x7B346F0, geturl_off: 0x78, seturl_off: 0x80F82A8 },
    FcurlOffsets { version: "++Fortnite+Release-27.00-CL-29072303", pr_offset: 0x840093C, geturl_off: 0x88, seturl_off: 0x8850D48 },
    FcurlOffsets { version: "++Fortnite+Release-27.10-CL-29703186", pr_offset: 0x8517AA4, geturl_off: 0x88, seturl_off: 0x893A5F4 },
    FcurlOffsets { version: "++Fortnite+Release-27.11-CL-29739262", pr_offset: 0x8F3F580, geturl_off: 0x88, seturl_off: 0x93527E4 },
    FcurlOffsets { version: "++Fortnite+Release-28.00-CL-29915848", pr_offset: 0x967E7FC, geturl_off: 0x150, seturl_off: 0x9A96B4C },
    FcurlOffsets { version: "++Fortnite+Release-28.01-CL-30342671", pr_offset: 0x968484C, geturl_off: 0x150, seturl_off: 0x9AADE34 },
    FcurlOffsets { version: "++Fortnite+Release-28.10-CL-30676362", pr_offset: 0x969A70C, geturl_off: 0x160, seturl_off: 0x9A8E0FC },
    FcurlOffsets { version: "++Fortnite+Release-28.10-CL-30835064", pr_offset: 0x969B098, geturl_off: 0x160, seturl_off: 0x9A8EA84 },
    FcurlOffsets { version: "++Fortnite+Release-28.20-CL-31165234", pr_offset: 0x98E27AC, geturl_off: 0x160, seturl_off: 0x9CBE7D4 },
    FcurlOffsets { version: "++Fortnite+Release-28.20-CL-31286935", pr_offset: 0x98E239C, geturl_off: 0x160, seturl_off: 0x9CBE3C8 },
    FcurlOffsets { version: "++Fortnite+Release-28.30-CL-31511038", pr_offset: 0x9914B88, geturl_off: 0x160, seturl_off: 0x9D11EF8 },
    FcurlOffsets { version: "++Fortnite+Release-29.00-CL-31978752", pr_offset: 0x50B8CC0, geturl_off: 0x208, seturl_off: 0x99FA714 },
    FcurlOffsets { version: "++Fortnite+Release-29.00-CL-32116959", pr_offset: 0x50B8C80, geturl_off: 0x208, seturl_off: 0x99F9D18 },
    FcurlOffsets { version: "++Fortnite+Release-29.00-CL-32148000", pr_offset: 0x50B8C74, geturl_off: 0x208, seturl_off: 0x99FA754 },
    FcurlOffsets { version: "++Fortnite+Release-29.01-CL-32236377", pr_offset: 0x50DAF3C, geturl_off: 0x208, seturl_off: 0x9A31F88 },
    FcurlOffsets { version: "++Fortnite+Release-29.10-CL-32391220", pr_offset: 0x512A56C, geturl_off: 0x218, seturl_off: 0x9B2326C },
    FcurlOffsets { version: "++Fortnite+Release-29.10-CL-32567225", pr_offset: 0x512B56C, geturl_off: 0x218, seturl_off: 0x9B2557C },
    FcurlOffsets { version: "++Fortnite+Release-29.20-CL-32716692", pr_offset: 0x518A638, geturl_off: 0x268, seturl_off: 0x9C7EDD4 },
    FcurlOffsets { version: "++Fortnite+Release-29.30-CL-32982357", pr_offset: 0x51D399C, geturl_off: 0x268, seturl_off: 0x9D246C0 },
    FcurlOffsets { version: "++Fortnite+Release-29.40-CL-33291686", pr_offset: 0x51CB330, geturl_off: 0x268, seturl_off: 0x9C72D10 },
    FcurlOffsets { version: "++Fortnite+Release-29.40-CL-33502036", pr_offset: 0x51CA330, geturl_off: 0x268, seturl_off: 0x9C6C3C4 },
    FcurlOffsets { version: "++Fortnite+Release-30.00-CL-33760522", pr_offset: 0x52C3300, geturl_off: 0x270, seturl_off: 0x9E875A0 },
    FcurlOffsets { version: "++Fortnite+Release-30.00-CL-33962396", pr_offset: 0x52C7E74, geturl_off: 0x270, seturl_off: 0x9E8D490 },
    FcurlOffsets { version: "++Fortnite+Release-30.10-CL-34184790", pr_offset: 0x535DD9C, geturl_off: 0x270, seturl_off: 0xA00AE98 },
    FcurlOffsets { version: "++Fortnite+Release-30.10-CL-34261954", pr_offset: 0x535DD9C, geturl_off: 0x270, seturl_off: 0xA00B1CC },
    FcurlOffsets { version: "++Fortnite+Release-30.20-CL-34488544", pr_offset: 0x53C1278, geturl_off: 0x270, seturl_off: 0xA0E7270 },
    FcurlOffsets { version: "++Fortnite+Release-30.20-CL-34597766", pr_offset: 0x53C2278, geturl_off: 0x270, seturl_off: 0xA0E83CC },
    FcurlOffsets { version: "++Fortnite+Release-30.30-CL-34891016", pr_offset: 0x5417020, geturl_off: 0x270, seturl_off: 0xA1ABA5C },
    FcurlOffsets { version: "++Fortnite+Release-30.40-CL-35235494", pr_offset: 0x54827DC, geturl_off: 0x270, seturl_off: 0xA2A3DCC },
    FcurlOffsets { version: "++Fortnite+Release-31.00-CL-35447195", pr_offset: 0x55B55C4, geturl_off: 0x270, seturl_off: 0xA5D7D44 },
    FcurlOffsets { version: "++Fortnite+Release-31.00-CL-35820075", pr_offset: 0x604A584, geturl_off: 0x270, seturl_off: 0xB37B56C },
    FcurlOffsets { version: "++Fortnite+Release-31.10-CL-35815136", pr_offset: 0x55DAA28, geturl_off: 0x270, seturl_off: 0xA62C754 },
    FcurlOffsets { version: "++Fortnite+Release-31.20-CL-36217724", pr_offset: 0x5631354, geturl_off: 0x270, seturl_off: 0xA6EBED8 },
    FcurlOffsets { version: "++Fortnite+Release-31.20-CL-36348035", pr_offset: 0x5631354, geturl_off: 0x270, seturl_off: 0xA6EBE18 },
    FcurlOffsets { version: "++Fortnite+Release-31.30-CL-36600465", pr_offset: 0x56A8774, geturl_off: 0x270, seturl_off: 0xA7FA914 },
    FcurlOffsets { version: "++Fortnite+Release-31.40-CL-36874825", pr_offset: 0x56C9590, geturl_off: 0x270, seturl_off: 0xA83D510 },
    FcurlOffsets { version: "++Fortnite+Release-31.40-CL-37076506", pr_offset: 0x56C9590, geturl_off: 0x270, seturl_off: 0xA83E7A8 },
    FcurlOffsets { version: "++Fortnite+Release-31.41-CL-37324991", pr_offset: 0x56D05B8, geturl_off: 0x270, seturl_off: 0xA84D3CC },
    FcurlOffsets { version: "++Fortnite+Release-32.00-CL-37505882", pr_offset: 0x59A6670, geturl_off: 0x288, seturl_off: 0xAB85574 },
    FcurlOffsets { version: "++Fortnite+Release-32.00-CL-37747778", pr_offset: 0x5963A70, geturl_off: 0x288, seturl_off: 0xAB357F8 },
    FcurlOffsets { version: "++Fortnite+Release-32.10-CL-37958378", pr_offset: 0x5974D60, geturl_off: 0x288, seturl_off: 0xAB5C23C },
    FcurlOffsets { version: "++Fortnite+Release-32.11-CL-38202817", pr_offset: 0x59A4E40, geturl_off: 0x288, seturl_off: 0xABBF324 },
    FcurlOffsets { version: "++Fortnite+Release-32.11-CL-38371047", pr_offset: 0x59AFD74, geturl_off: 0x288, seturl_off: 0xABBFC48 },
    FcurlOffsets { version: "++Fortnite+Release-33.00-CL-38416954", pr_offset: 0x5A42DF0, geturl_off: 0x288, seturl_off: 0xAE0E770 },
    FcurlOffsets { version: "++Fortnite+Release-33.00-CL-38504598", pr_offset: 0x5A36F70, geturl_off: 0x288, seturl_off: 0xAD9CE6C },
];

/// FCurlHttpRequest::ProcessRequest offsets inside libEOSSDK.so.
pub const EOS_FCURL_OFFSETS: &[FcurlOffsets] = &[
    FcurlOffsets { version: "++Fortnite+Release-22.00-CL-22031344", pr_offset: 0xDEC088, geturl_off: 0x178, seturl_off: 0xDE8A3C },
    FcurlOffsets { version: "++Fortnite+Release-22.00-CL-22107157", pr_offset: 0xDEC088, geturl_off: 0x178, seturl_off: 0xDE8A3C },
    FcurlOffsets { version: "++Fortnite+Release-22.10-CL-22240570", pr_offset: 0xF20DC4, geturl_off: 0x178, seturl_off: 0xF1D778 },
    FcurlOffsets { version: "++Fortnite+Release-22.20-CL-22545427", pr_offset: 0xF28A34, geturl_off: 0x178, seturl_off: 0xF253E8 },
    FcurlOffsets { version: "++Fortnite+Release-22.30-CL-22854046", pr_offset: 0xF2D508, geturl_off: 0x178, seturl_off: 0xF29EBC },
    FcurlOffsets { version: "++Fortnite+Release-22.30-CL-22886174", pr_offset: 0xF2D508, geturl_off: 0x178, seturl_off: 0xF29EBC },
    FcurlOffsets { version: "++Fortnite+Release-22.40-CL-23070899", pr_offset: 0xFA881C, geturl_off: 0x178, seturl_off: 0xFA51D0 },
    FcurlOffsets { version: "++Fortnite+Release-23.00-CL-23281232", pr_offset: 0xFA885C, geturl_off: 0x178, seturl_off: 0xFA5210 },
    FcurlOffsets { version: "++Fortnite+Release-23.00-CL-23385779", pr_offset: 0xFA885C, geturl_off: 0x178, seturl_off: 0xFA5210 },
    FcurlOffsets { version: "++Fortnite+Release-23.10-CL-23443094", pr_offset: 0xFA885C, geturl_off: 0x178, seturl_off: 0xFA5210 },
    FcurlOffsets { version: "++Fortnite+Release-23.10-CL-23572221", pr_offset: 0xFA885C, geturl_off: 0x178, seturl_off: 0xFA5210 },
    FcurlOffsets { version: "++Fortnite+Release-23.20-CL-23659353", pr_offset: 0xFAC0F0, geturl_off: 0x178, seturl_off: 0xFA8AA4 },
    FcurlOffsets { version: "++Fortnite+Release-23.20-CL-23783097", pr_offset: 0xFAC0F0, geturl_off: 0x178, seturl_off: 0xFA8AA4 },
    FcurlOffsets { version: "++Fortnite+Release-23.30-CL-23901854", pr_offset: 0xF70A5C, geturl_off: 0x178, seturl_off: 0xF6D410 },
    FcurlOffsets { version: "++Fortnite+Release-23.30-CL-23937783", pr_offset: 0xF70A5C, geturl_off: 0x178, seturl_off: 0xF6D410 },
    FcurlOffsets { version: "++Fortnite+Release-23.30-CL-23986860", pr_offset: 0xF70A5C, geturl_off: 0x178, seturl_off: 0xF6D410 },
    FcurlOffsets { version: "++Fortnite+Release-23.40-CL-24087481", pr_offset: 0xF70A5C, geturl_off: 0x178, seturl_off: 0xF6D410 },
    FcurlOffsets { version: "++Fortnite+Release-23.40-CL-24230880", pr_offset: 0xF70A5C, geturl_off: 0x178, seturl_off: 0xF6D410 },
    FcurlOffsets { version: "++Fortnite+Release-23.50-CL-24376996", pr_offset: 0xF880E4, geturl_off: 0x178, seturl_off: 0xF84A98 },
    FcurlOffsets { version: "++Fortnite+Release-24.00-CL-24518431", pr_offset: 0xF8A590, geturl_off: 0x178, seturl_off: 0xF86F44 },
    FcurlOffsets { version: "++Fortnite+Release-24.01-CL-24665656", pr_offset: 0xF8A590, geturl_off: 0x178, seturl_off: 0xF86F44 },
    FcurlOffsets { version: "++Fortnite+Release-24.10-CL-24770548", pr_offset: 0xF8A590, geturl_off: 0x178, seturl_off: 0xF86F44 },
    FcurlOffsets { version: "++Fortnite+Release-24.20-CL-24939793", pr_offset: 0xFB2DFC, geturl_off: 0x178, seturl_off: 0xFAF7B0 },
    FcurlOffsets { version: "++Fortnite+Release-24.30-CL-25280217", pr_offset: 0xFB2DFC, geturl_off: 0x178, seturl_off: 0xFAF7B0 },
    FcurlOffsets { version: "++Fortnite+Release-24.30-CL-25330280", pr_offset: 0xFB2DFC, geturl_off: 0x178, seturl_off: 0xFAF7B0 },
    FcurlOffsets { version: "++Fortnite+Release-24.40-CL-25469380", pr_offset: 0xFB2DFC, geturl_off: 0x178, seturl_off: 0xFAF7B0 },
    FcurlOffsets { version: "++Fortnite+Release-24.40-CL-25595478", pr_offset: 0xFB2DFC, geturl_off: 0x178, seturl_off: 0xFAF7B0 },
    FcurlOffsets { version: "++Fortnite+Release-25.00-CL-25695576", pr_offset: 0xFB2DFC, geturl_off: 0x178, seturl_off: 0xFAF7B0 },
    FcurlOffsets { version: "++Fortnite+Release-25.00-CL-25913641", pr_offset: 0xFB2DFC, geturl_off: 0x178, seturl_off: 0xFAF7B0 },
    FcurlOffsets { version: "++Fortnite+Release-25.10-CL-26000958", pr_offset: 0xFD14D8, geturl_off: 0x178, seturl_off: 0xFCDE8C },
    FcurlOffsets { version: "++Fortnite+Release-25.11-CL-26227037", pr_offset: 0xFD14D8, geturl_off: 0x178, seturl_off: 0xFCDE8C },
    FcurlOffsets { version: "++Fortnite+Release-25.20-CL-26474516", pr_offset: 0xFD31D4, geturl_off: 0x178, seturl_off: 0xFCFB88 },
    FcurlOffsets { version: "++Fortnite+Release-25.20-CL-26643816", pr_offset: 0xFD31D4, geturl_off: 0x178, seturl_off: 0xFCFB88 },
    FcurlOffsets { version: "++Fortnite+Release-25.30-CL-26867995", pr_offset: 0xFF5118, geturl_off: 0x178, seturl_off: 0xFF1ACC },
    FcurlOffsets { version: "++Fortnite+Release-26.00-CL-27233190", pr_offset: 0x1030348, geturl_off: 0x178, seturl_off: 0x102CCFC },
    FcurlOffsets { version: "++Fortnite+Release-26.00-CL-27405594", pr_offset: 0x1030348, geturl_off: 0x178, seturl_off: 0x102CCFC },
    FcurlOffsets { version: "++Fortnite+Release-26.10-CL-27665530", pr_offset: 0x1031F34, geturl_off: 0x178, seturl_off: 0x102E8E8 },
    FcurlOffsets { version: "++Fortnite+Release-26.20-CL-28096793", pr_offset: 0xFDCB40, geturl_off: 0x178, seturl_off: 0xFD94F4 },
    FcurlOffsets { version: "++Fortnite+Release-26.20-CL-28333111", pr_offset: 0xFDCB40, geturl_off: 0x178, seturl_off: 0xFD94F4 },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28520140", pr_offset: 0x104EC18, geturl_off: 0x178, seturl_off: 0x104B5CC },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28620401", pr_offset: 0x104EC18, geturl_off: 0x178, seturl_off: 0x104B5CC },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28688692", pr_offset: 0x104F0B4, geturl_off: 0x178, seturl_off: 0x104BA68 },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28849396", pr_offset: 0x104F0B4, geturl_off: 0x178, seturl_off: 0x104BA68 },
    FcurlOffsets { version: "++Fortnite+Release-26.30-CL-28938851", pr_offset: 0x104F0B4, geturl_off: 0x178, seturl_off: 0x104BA68 },
    FcurlOffsets { version: "++Fortnite+Release-27.00-CL-29072303", pr_offset: 0x104F0B4, geturl_off: 0x178, seturl_off: 0x104BA68 },
    FcurlOffsets { version: "++Fortnite+Release-27.10-CL-29703186", pr_offset: 0x1052524, geturl_off: 0x178, seturl_off: 0x104EED8 },
    FcurlOffsets { version: "++Fortnite+Release-27.11-CL-29739262", pr_offset: 0x1052524, geturl_off: 0x178, seturl_off: 0x104EED8 },
    FcurlOffsets { version: "++Fortnite+Release-28.00-CL-29915848", pr_offset: 0x10D7BDC, geturl_off: 0x178, seturl_off: 0x10D4590 },
    FcurlOffsets { version: "++Fortnite+Release-28.01-CL-30342671", pr_offset: 0x10D6C0C, geturl_off: 0x178, seturl_off: 0x10D35C0 },
    FcurlOffsets { version: "++Fortnite+Release-28.10-CL-30676362", pr_offset: 0x10EF380, geturl_off: 0x178, seturl_off: 0x10EBD34 },
    FcurlOffsets { version: "++Fortnite+Release-28.10-CL-30835064", pr_offset: 0x10EF380, geturl_off: 0x178, seturl_off: 0x10EBD34 },
    FcurlOffsets { version: "++Fortnite+Release-28.20-CL-31165234", pr_offset: 0x110335C, geturl_off: 0x178, seturl_off: 0x10FFD10 },
    FcurlOffsets { version: "++Fortnite+Release-28.20-CL-31286935", pr_offset: 0x110335C, geturl_off: 0x178, seturl_off: 0x10FFD10 },
    FcurlOffsets { version: "++Fortnite+Release-28.30-CL-31511038", pr_offset: 0x1105254, geturl_off: 0x178, seturl_off: 0x1101C08 },
    FcurlOffsets { version: "++Fortnite+Release-29.00-CL-31978752", pr_offset: 0x1104044, geturl_off: 0x178, seturl_off: 0x11009F8 },
    FcurlOffsets { version: "++Fortnite+Release-29.00-CL-32116959", pr_offset: 0x1104044, geturl_off: 0x178, seturl_off: 0x11009F8 },
    FcurlOffsets { version: "++Fortnite+Release-29.00-CL-32148000", pr_offset: 0x1104044, geturl_off: 0x178, seturl_off: 0x11009F8 },
    FcurlOffsets { version: "++Fortnite+Release-29.01-CL-32236377", pr_offset: 0x1104044, geturl_off: 0x178, seturl_off: 0x11009F8 },
    FcurlOffsets { version: "++Fortnite+Release-29.10-CL-32391220", pr_offset: 0x103C6A8, geturl_off: 0x178, seturl_off: 0x103905C },
    FcurlOffsets { version: "++Fortnite+Release-29.10-CL-32567225", pr_offset: 0x103C6A8, geturl_off: 0x178, seturl_off: 0x103905C },
    FcurlOffsets { version: "++Fortnite+Release-29.20-CL-32716692", pr_offset: 0x10422CC, geturl_off: 0x178, seturl_off: 0x103EC80 },
    FcurlOffsets { version: "++Fortnite+Release-29.30-CL-32982357", pr_offset: 0x105B2C8, geturl_off: 0x178, seturl_off: 0x1057C7C },
    FcurlOffsets { version: "++Fortnite+Release-29.40-CL-33291686", pr_offset: 0x10A8028, geturl_off: 0x178, seturl_off: 0x10A49DC },
    FcurlOffsets { version: "++Fortnite+Release-29.40-CL-33502036", pr_offset: 0x10A8028, geturl_off: 0x178, seturl_off: 0x10A49DC },
    FcurlOffsets { version: "++Fortnite+Release-30.00-CL-33760522", pr_offset: 0x10A8148, geturl_off: 0x178, seturl_off: 0x10A4AFC },
    FcurlOffsets { version: "++Fortnite+Release-30.00-CL-33962396", pr_offset: 0x10A8148, geturl_off: 0x178, seturl_off: 0x10A4AFC },
    FcurlOffsets { version: "++Fortnite+Release-30.10-CL-34184790", pr_offset: 0x10ADA38, geturl_off: 0x178, seturl_off: 0x10AA3EC },
    FcurlOffsets { version: "++Fortnite+Release-30.10-CL-34261954", pr_offset: 0x10ADA38, geturl_off: 0x178, seturl_off: 0x10AA3EC },
    FcurlOffsets { version: "++Fortnite+Release-30.20-CL-34488544", pr_offset: 0x10BA2DC, geturl_off: 0x178, seturl_off: 0x10B6C90 },
    FcurlOffsets { version: "++Fortnite+Release-30.20-CL-34597766", pr_offset: 0x10BA2DC, geturl_off: 0x178, seturl_off: 0x10B6C90 },
    FcurlOffsets { version: "++Fortnite+Release-30.30-CL-34891016", pr_offset: 0x10BA2E0, geturl_off: 0x178, seturl_off: 0x10B6C94 },
    FcurlOffsets { version: "++Fortnite+Release-30.40-CL-35235494", pr_offset: 0x10BA2DC, geturl_off: 0x178, seturl_off: 0x10B6C90 },
    FcurlOffsets { version: "++Fortnite+Release-31.00-CL-35447195", pr_offset: 0x10BA2DC, geturl_off: 0x178, seturl_off: 0x10B6C90 },
    FcurlOffsets { version: "++Fortnite+Release-31.00-CL-35820075", pr_offset: 0x10BA2DC, geturl_off: 0x178, seturl_off: 0x10B6C90 },
    FcurlOffsets { version: "++Fortnite+Release-31.10-CL-35815136", pr_offset: 0x10BA2DC, geturl_off: 0x178, seturl_off: 0x10B6C90 },
    FcurlOffsets { version: "++Fortnite+Release-31.20-CL-36217724", pr_offset: 0x10DAC84, geturl_off: 0x178, seturl_off: 0x10D7638 },
    FcurlOffsets { version: "++Fortnite+Release-31.20-CL-36348035", pr_offset: 0x10DAC84, geturl_off: 0x178, seturl_off: 0x10D7638 },
    FcurlOffsets { version: "++Fortnite+Release-31.30-CL-36600465", pr_offset: 0x1141D90, geturl_off: 0x178, seturl_off: 0x113E744 },
    FcurlOffsets { version: "++Fortnite+Release-31.40-CL-36874825", pr_offset: 0x11FBB34, geturl_off: 0x178, seturl_off: 0x11F84E8 },
    FcurlOffsets { version: "++Fortnite+Release-31.40-CL-37076506", pr_offset: 0x11FBB34, geturl_off: 0x178, seturl_off: 0x11F84E8 },
    FcurlOffsets { version: "++Fortnite+Release-31.41-CL-37324991", pr_offset: 0x11FBB34, geturl_off: 0x178, seturl_off: 0x11F84E8 },
    FcurlOffsets { version: "++Fortnite+Release-32.00-CL-37505882", pr_offset: 0x11FBB34, geturl_off: 0x178, seturl_off: 0x11F84E8 },
    FcurlOffsets { version: "++Fortnite+Release-32.00-CL-37747778", pr_offset: 0x11FBB34, geturl_off: 0x178, seturl_off: 0x11F84E8 },
    FcurlOffsets { version: "++Fortnite+Release-32.10-CL-37958378", pr_offset: 0x125F9A4, geturl_off: 0x178, seturl_off: 0x125C358 },
    FcurlOffsets { version: "++Fortnite+Release-32.11-CL-38202817", pr_offset: 0x125F9A4, geturl_off: 0x178, seturl_off: 0x125C358 },
    FcurlOffsets { version: "++Fortnite+Release-32.11-CL-38371047", pr_offset: 0x125F9A4, geturl_off: 0x178, seturl_off: 0x125C358 },
    FcurlOffsets { version: "++Fortnite+Release-33.00-CL-38416954", pr_offset: 0x125F9A4, geturl_off: 0x178, seturl_off: 0x125C358 },
    FcurlOffsets { version: "++Fortnite+Release-33.00-CL-38504598", pr_offset: 0x125F9A4, geturl_off: 0x178, seturl_off: 0x125C358 },
];

/// Kairos WebView era: 10.31 (frontend rewrite) through 15.50 (last Kairos
/// build on mobile).
pub const KAIROS_RANGE: ((u32, u32), (u32, u32)) = ((10, 31), (15, 50));

fn version_tuple(v: &FortniteVersion) -> (u32, u32) {
    (v.major, v.minor)
}

/// Pick the primary redirect method for a version.
pub fn method_for(v: &FortniteVersion) -> RedirectMethod {
    let t = version_tuple(v);
    if t >= KAIROS_RANGE.0 && t <= KAIROS_RANGE.1 {
        RedirectMethod::KairosWeb
    } else if t > KAIROS_RANGE.1 {
        RedirectMethod::NativeCurl
    } else {
        RedirectMethod::ExportedCurl
    }
}

/// Look up legacy FCurl offsets (libUnreal.so) for a release string.
pub fn ue_fcurl_offsets(release: &str) -> Option<&'static FcurlOffsets> {
    UE_FCURL_OFFSETS.iter().find(|o| o.version == release)
}

/// Look up legacy FCurl offsets (libEOSSDK.so) for a release string.
pub fn eos_fcurl_offsets(release: &str) -> Option<&'static FcurlOffsets> {
    EOS_FCURL_OFFSETS.iter().find(|o| o.version == release)
}

/// Everything the redirecter needs to know about one detected build.
#[derive(Debug, Clone, Serialize)]
pub struct VersionProfile {
    pub version: FortniteVersion,
    pub method: RedirectMethod,
    pub ue_fcurl: Option<&'static FcurlOffsets>,
    pub eos_fcurl: Option<&'static FcurlOffsets>,
    /// True when the Kairos classes (AuthFragment / KernelImpl /
    /// DebouncedLimiter) are expected in the APK.
    pub kairos_webview: bool,
}

pub fn profile_for(version: FortniteVersion) -> VersionProfile {
    let method = method_for(&version);
    VersionProfile {
        ue_fcurl: ue_fcurl_offsets(&version.release_string),
        eos_fcurl: eos_fcurl_offsets(&version.release_string),
        kairos_webview: method == RedirectMethod::KairosWeb,
        version,
        method,
    }
}

/// A catalog row for the web panel: which versions Owen supports and how.
pub fn version_catalog() -> serde_json::Value {
    use serde_json::json;
    let rows: Vec<(u32, u32)> = vec![
        (10, 31), (10, 40), (10, 41), (11, 0), (11, 1), (11, 11), (11, 31),
        (12, 0), (12, 10), (12, 21), (12, 30), (12, 41), (12, 50), (12, 60),
        (12, 61), (13, 0), (13, 20), (13, 30), (13, 40), (14, 0), (14, 10),
        (14, 20), (14, 30), (14, 40), (14, 50), (14, 60), (15, 0), (15, 10),
        (15, 20), (15, 30), (15, 40), (15, 50), (16, 0), (16, 40), (16, 50),
        (17, 0), (17, 30), (17, 50), (18, 0), (18, 40), (19, 0), (19, 10),
        (19, 40), (20, 0), (20, 10), (20, 20), (20, 30), (20, 40), (21, 0),
        (21, 30), (21, 50), (22, 0), (22, 40), (23, 0), (23, 50), (24, 0),
        (25, 0), (26, 0), (27, 0), (28, 0), (29, 0), (29, 30), (30, 0),
        (31, 0), (31, 30), (32, 0), (33, 0),
    ];
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|&(major, minor)| {
            let ver = crate::version::FortniteVersion {
                major,
                minor,
                cl: 0,
                release_string: format!("++Fortnite+Release-{major}.{minor:02}"),
            };
            let method = method_for(&ver);
            let (chapter, season) = ver.season();
            let method_label = match method {
                RedirectMethod::KairosWeb => "Kairos WebView (ART hooks: DebouncedLimiter + WebView.loadUrl; no smali)",
                RedirectMethod::NativeCurl => "Native (auto-detected curl_easy_setopt in libUnreal.so + libEOSSDK.so)",
                RedirectMethod::ExportedCurl => "Native (exported curl_easy_setopt in libUE4.so)",
                RedirectMethod::Auto => "Automatic (all methods attempted)",
            };
            json!({
                "version": format!("{major}.{minor:02}"),
                "chapter": chapter,
                "season": season,
                "method": method_label,
                "method_key": method,
                "kairos_webview": method == RedirectMethod::KairosWeb,
                "partyhub": method == RedirectMethod::KairosWeb,
                "fcurl_offsets": ue_fcurl_offsets(&ver.release_string).is_some(),
            })
        })
        .collect();
    json!({
        "total": entries.len(),
        "versions": entries,
        "notes": [
            "Kairos WebView era (10.31-15.50): login and PartyHub graphql are redirected by ART-hooking com.epicgames.kairos.utils.DebouncedLimiter inside the game process. Zero smali edits.",
            "Native era (16.00+): all HTTP flows through libcurl; curl_easy_setopt is located by the automatic scanner and hooked once.",
            "libUE4.so era: curl_easy_setopt is an exported symbol and is resolved directly.",
            "Both http:// and https:// backends are accepted for every version.",
        ],
    })
}
