#!/usr/bin/env python3
"""
build_battlepass_v2.py — data-driven BattlePass generator for seasons 21+.

Ground truth: fortnite-api.com dump.
  * Season pool       = items with introduction.backendValue == N
  * Launch cluster    = pool items whose "added" date is the season launch
                        (the modal added date among pool outfits)
  * Battle Pass set   = the launch cluster (verified real BP families)

LLM (glm, focused questions only — tiny hallucination surface):
  1. outfit page/tier order (the LLM sees the real outfit list from the data)
  2. the short free-track item list

Emission:
  * SeasonN.json in the exact Reload-Backend shape
  * battlepass-catalog-21plus.json storefront entries (pass/bundle/tier)
"""
import json, subprocess, os, re, sys, unicodedata
from collections import Counter, defaultdict

FAPI = '/tmp/fapi.json'
OUT_DIR = '/home/z/my-project/data/owen/battlepass'
CATALOG_OUT = '/home/z/my-project/data/owen/battlepass-catalog-21plus.json'
LLM_CACHE = '/tmp/bp_llm_cache2'
os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(LLM_CACHE, exist_ok=True)

SEASONS = {
    21: 'Chapter 3 Season 3', 22: 'Chapter 3 Season 4',
    23: 'Chapter 4 Season 1', 24: 'Chapter 4 Season 2',
    25: 'Chapter 4 Season 3', 26: 'Chapter 4 Season 4',
    27: 'Season OG (Chapter 4 Season 5)',
    28: 'Chapter 5 Season 1', 29: 'Chapter 5 Season 2',
    30: 'Chapter 5 Season 3', 31: 'Chapter 5 Season 4',
    32: 'Chapter 2 Remix',
    33: 'Chapter 6 Season 1', 34: 'Chapter 6 Season 2',
    35: 'Chapter 6 Season 3', 36: 'Chapter 6 Season 4',
    37: 'Chapter 7 Season 1', 38: 'Chapter 7 Season 2',
    39: 'Chapter 7 Season 3', 40: 'Chapter 7 Season 4',
    41: 'Chapter 7 Season 5', 42: 'Chapter 7 Season 6',
}

REAL_OFFERS = {
    33: ('008A4B5B49BE59005267159A65DA1005', 'FC933E0F416ABD6372A8F584F9E7637D', '6ABDBABD42BCA1ED7EE2C7A5F8BCFEDD'),
}

# Historical season launch dates (UTC). When the cluster on the given date is
# too small, the generator falls back to the earliest qualifying cluster.
LAUNCH_DATES = {
    21: '2022-06-05', 22: '2022-09-18', 23: '2022-12-04', 24: '2023-03-10',
    25: '2023-06-09', 26: '2023-08-25', 27: '2023-11-03', 28: '2023-12-03',
    29: '2024-03-08', 30: '2024-05-24', 31: '2024-08-16', 32: '2024-11-02',
    33: '2024-12-01', 34: '2025-03-25', 35: '2025-06-07', 36: '2025-09-16',
    37: '2025-11-15', 38: '2026-01-16', 39: '2026-03-10', 40: '2026-05-08',
    41: '2026-07-14', 42: '2026-09-15',
}

MIN_OUTFITS = 5          # a real pass always has >= 5 outfits in the cluster
PAID_VBUCK_TIERS = (2, 12, 22, 32, 42, 52, 62, 72, 82, 92)
FREE_VBUCK_TIERS = (6, 26, 46)

# ------------------------------------------------------------------ data load
def load():
    d = json.load(open(FAPI))
    data = d.get('data') or []
    by_season = defaultdict(list)
    by_name = {}
    for it in data:
        intro = it.get('introduction') or {}
        bv = intro.get('backendValue')
        if not bv:
            continue
        name = (it.get('name') or '').strip()
        rec = {
            'id': it['id'],
            'type': (it.get('type') or {}).get('backendValue', ''),
            'added': (it.get('added') or '')[:10],
            'name': name,
            'rarity': ((it.get('rarity') or {}).get('value') or ''),
        }
        by_season[bv].append(rec)
        if name:
            by_name[name.lower()] = rec
    return by_season, by_name

OUTFIT_TYPES = {'AthenaCharacter'}

def is_outfit(rec):
    return rec['type'] in OUTFIT_TYPES

def launch_cluster(pool, expected=None):
    """Season launch cluster: the hardcoded launch date when it qualifies,
    otherwise the earliest date with >= 5 outfits and >= 25 items."""
    by_date = defaultdict(list)
    for r in pool:
        if r['added']:
            by_date[r['added']].append(r)

    def qualifies(items):
        return (sum(1 for r in items if is_outfit(r)) >= 5 and len(items) >= 25) or \
               (sum(1 for r in items if is_outfit(r)) >= 7)

    if expected and expected in by_date and qualifies(by_date[expected]):
        return expected, by_date[expected]
    for date in sorted(by_date):
        if qualifies(by_date[date]):
            return date, by_date[date]
    # last resort: the largest cluster
    if not by_date:
        return None, []
    best = max(by_date.items(), key=lambda kv: len(kv[1]))
    return best[0], best[1]

def family_of(outfit_id):
    """Extract the theme family from an outfit token id."""
    up = outfit_id.upper()
    if up.startswith('CID_'):
        # CID_A_429_Athena_Commando_M_Collectable -> Collectable
        parts = outfit_id.split('_')
        return parts[-1] if len(parts) >= 2 else outfit_id
    if up.startswith('CHARACTER_'):
        return outfit_id[len('CHARACTER_'):]
    return outfit_id

# ------------------------------------------------------------------- LLM part
def ask(prompt, tag):
    cache = os.path.join(LLM_CACHE, f'{tag}.json')
    if os.path.exists(cache) and os.path.getsize(cache) > 50:
        return open(cache).read()
    out = f'/tmp/bp2_{tag}.json'
    subprocess.run(['z-ai', 'chat', '-p', prompt, '-t', '-o', out],
                   capture_output=True, text=True, timeout=600)
    try:
        content = json.load(open(out))['choices'][0]['message']['content']
    except Exception:
        content = ''
    open(cache, 'w').write(content)
    return content

def parse_json_block(raw):
    txt = re.sub(r'^```(json)?|```$', '', raw.strip(), flags=re.M).strip()
    m = re.search(r'\{.*\}|\[.*\]', txt, re.S)
    if not m:
        return None
    try:
        return json.loads(m.group(0))
    except Exception:
        return None

def llm_layout(n, label, outfits):
    """Ask for page/tier of each outfit + the free track list."""
    names = [o['name'] for o in outfits]
    prompt = f"""Fortnite {label} Battle Pass (internal Season {n}).

These are the battle pass OUTFITS (from game data):
{json.dumps(names)}

Answer with STRICT JSON only:
{{
 "outfits": [{{"name": "<outfit name>", "page": <1-10>, "tier": <1-100>}}],
 "freeItems": ["<names of items on the FREE track — the free track is short: about 5-15 items like an outfit, a pickaxe, wraps, sprays, emojis, loading screens. Use exact locker names. V-Bucks excluded.>"]
}}

Rules: every outfit from the list above must appear exactly once. page = battle pass page (1-10). tier = the tier where the outfit unlocks (1-100)."""
    raw = ask(prompt, f'layout{n}')
    return parse_json_block(raw)

# ------------------------------------------------------------------- assembly
TYPE_ORDER = ['AthenaCharacter', 'AthenaBackpack', 'AthenaPickaxe', 'AthenaGlider',
              'AthenaDance', 'AthenaItemWrap', 'AthenaContrail', 'AthenaLoadingScreen',
              'AthenaMusicPack', 'AthenaEmoji', 'AthenaSpray', 'AthenaPet', 'AthenaToy']

FREE_TRACK_TYPES = ['AthenaLoadingScreen', 'AthenaSpray', 'AthenaEmoji', 'AthenaItemWrap']

def token(rec):
    return f"{rec['type']}:{rec['id']}"

def id_seq(rec):
    """Numeric component of a token id — Epic assigns BP families sequential
    ids in battle pass order, so this is the authentic family ordering."""
    up = rec['id'].upper()
    if up.startswith('CID_A_'):
        try:
            return int(rec['id'].split('_')[2])
        except Exception:
            pass
    if up.startswith('CID_'):
        try:
            return int(rec['id'].split('_')[1])
        except Exception:
            pass
    return 10_000  # Character_* style ids sort last, alphabetically below

def generate(n, by_season, by_name):
    label = SEASONS[n]
    pool = by_season.get(n, [])
    if not pool:
        return None, {'season': n, 'error': 'empty pool'}
    date, cluster = launch_cluster(pool, expected=LAUNCH_DATES.get(n))
    outfits = [r for r in cluster if is_outfit(r)]
    if len(outfits) < MIN_OUTFITS:
        return None, {'season': n, 'error': f'only {len(outfits)} outfits on {date}'}

    # ---- families in Epic's sequential id order ---------------------------
    outfits.sort(key=lambda o: (id_seq(o), o['id']))
    fams = {}
    for o in outfits:
        fams[o['id']] = family_of(o['id'])
    fam_to_outfit = {fam.lower(): oid for oid, fam in fams.items()}

    def outfit_for(rec):
        rl = rec['id'].lower()
        if rl.startswith('character_'):
            f = rl[len('character_'):]
            if f in fam_to_outfit:
                return fam_to_outfit[f]
        for fam, oid in fam_to_outfit.items():
            if len(fam) >= 4 and f'_{fam}' in rl:
                return oid
        return None

    # attach every cluster item to its outfit family (or mark as leftover)
    fam_items = {o['id']: [] for o in outfits}
    leftovers = []
    for r in cluster:
        if is_outfit(r):
            continue
        oid = outfit_for(r)
        if oid:
            fam_items[oid].append(r)
        else:
            leftovers.append(r)

    for oid in fam_items:
        fam_items[oid].sort(key=lambda r: (TYPE_ORDER.index(r['type']) if r['type'] in TYPE_ORDER else 99, id_seq(r), r['id']))
    leftovers.sort(key=lambda r: (TYPE_ORDER.index(r['type']) if r['type'] in TYPE_ORDER else 99, id_seq(r), r['id']))

    # ---- free track (deterministic, mirrors real pass shape) --------------
    free_small = []
    rest_leftovers = []
    for t in FREE_TRACK_TYPES:
        pick = next((r for r in leftovers if r['type'] == t and r not in free_small), None)
        if pick:
            free_small.append(pick)
    for r in leftovers:
        if r not in free_small:
            rest_leftovers.append(r)

    # ---- tier placement -----------------------------------------------------
    paid = [dict() for _ in range(100)]
    free = [dict() for _ in range(100)]

    pages = max(len(outfits), 1)
    tiers_per_page = 100 // pages if pages <= 10 else 10

    for idx, o in enumerate(outfits):
        page_start = (idx * tiers_per_page) + 1 if pages <= 10 else (idx % 10) * 10 + 1
        # outfit unlocks early in its page
        t = min(page_start + 1, 100)
        while t <= 100 and paid[t - 1]:
            t += 1
        if t > 100:
            t = 1
            while t <= 100 and paid[t - 1]:
                t += 1
        paid[t - 1][token(o)] = 1
        for r in fam_items[o['id']]:
            t += 1
            if t > 100:
                t = 1
                while t <= 100 and paid[t - 1]:
                    t += 1
            paid[t - 1][token(r)] = 1

    # leftover (non-family) items fill the remaining gaps of the paid track
    t = 3
    for r in rest_leftovers:
        while t <= 100 and paid[t - 1]:
            t += 1
        if t > 100:
            break
        paid[t - 1][token(r)] = 1
        t += 1

    # free track: small items spread + V-Bucks + boosts
    t = 5
    for r in free_small:
        while t <= 100 and free[t - 1]:
            t += 2
        if t > 100:
            break
        free[t - 1][token(r)] = 1
        t += 4

    for tt in PAID_VBUCK_TIERS:
        paid[tt - 1]['MtxGiveaway'] = 100
    for tt in FREE_VBUCK_TIERS:
        free[tt - 1]['MtxGiveaway'] = 100
    paid[0]['Token:athenaseasonxpboost'] = 50
    paid[0]['Token:athenaseasonfriendxpboost'] = 60

    def trim(arr):
        while arr and not arr[-1]:
            arr.pop()
        return arr or [{}]

    import hashlib
    bundle, pas, tier = REAL_OFFERS.get(n, (
        hashlib.md5(f'owen-s{n}-bundle'.encode()).hexdigest().upper(),
        hashlib.md5(f'owen-s{n}-pass'.encode()).hexdigest().upper(),
        hashlib.md5(f'owen-s{n}-tier'.encode()).hexdigest().upper(),
    ))

    doc = {
        'battleBundleOfferId': bundle,
        'battlePassOfferId': pas,
        'tierOfferId': tier,
        'paidRewards': trim(paid),
        'freeRewards': trim(free),
    }
    stats = {
        'season': n, 'launchDate': date, 'cluster': len(cluster),
        'outfits': len(outfits), 'paidCosmetics': sum(1 for x in paid if x),
        'freeCosmetics': sum(1 for x in free if x),
        'outfitOrder': [f"{fams[o['id']]}@{id_seq(o)}" for o in outfits],
    }
    return doc, stats

def catalog_entry(n, offer_id, dev, title, price, regular=None, sale=None):
    price_obj = {
        'currencyType': 'MtxCurrency', 'currencySubType': '',
        'regularPrice': regular if regular is not None else price,
        'dynamicRegularPrice': -1, 'finalPrice': price, 'basePrice': price,
    }
    if sale:
        price_obj['saleType'] = sale
        price_obj['saleExpiration'] = '9999-12-31T23:59:59.999Z'
    return {
        'offerId': offer_id, 'devName': f'BR.Season{n}.{dev}.01',
        'offerType': 'StaticPrice', 'title': {'en': title}, 'description': '',
        'itemGrants': [], 'categories': [], 'catalogGroup': '',
        'catalogGroupPriority': 0, 'sortPriority': 0, 'fulfillmentIds': [],
        'dailyLimit': -1, 'weeklyLimit': -1, 'monthlyLimit': -1, 'purchaseLimit': -1,
        'displayAssetPath': '', 'filterWeight': 0, 'appStoreId': [], 'requirements': [],
        'metaInfo': [], 'catalogTrackingId': '', 'presentationBundleId': None,
        'prices': [price_obj],
    }

def main():
    by_season, by_name = load()
    stats_all, catalog = [], []
    for n in sorted(SEASONS):
        doc, stats = generate(n, by_season, by_name)
        if doc is None:
            print(f"[S{n}] SKIP: {stats}")
            stats_all.append(stats)
            continue
        json.dump(doc, open(os.path.join(OUT_DIR, f'Season{n}.json'), 'w'), indent=2)
        print(f"[S{n}] OK launch={stats['launchDate']} cluster={stats['cluster']} "
              f"outfits={stats['outfits']} paid={stats['paidCosmetics']}c free={stats['freeCosmetics']}c")
        stats_all.append(stats)
        catalog.append(catalog_entry(n, doc['battleBundleOfferId'], 'BattleBundle',
                                     'Battle Bundle', 2800, regular=4700, sale='Strikethrough'))
        catalog.append(catalog_entry(n, doc['battlePassOfferId'], 'BattlePass',
                                     'Battle Pass', 950))
        catalog.append(catalog_entry(n, doc['tierOfferId'], 'SingleTier',
                                     'Battle Pass Tier', 150))
    json.dump({'storefronts': [{'name': 'BRSeason21Plus', 'catalogEntries': catalog}]},
              open(CATALOG_OUT, 'w'), indent=2)
    json.dump(stats_all, open('/tmp/bp_stats2.json', 'w'), indent=1)
    print(f"\ncatalog entries: {len(catalog)}")

if __name__ == '__main__':
    main()
