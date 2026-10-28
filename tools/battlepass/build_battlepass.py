#!/usr/bin/env python3
"""
build_battlepass.py — generate Reload/Lawin-compatible BattlePass/SeasonN.json
for internal seasons 21..42 (Chapter 3 Season 3 onward).

Pipeline:
  1. fortnite-api.com dump  = ground truth for token ids + intro season.
  2. GLM (z-ai CLI, thinking)= per-season battle pass enumeration (names, tier,
                              free/paid track).
  3. Verification            = every cosmetic must exist in fortnite-api AND
                              have introduction.backendValue == season.
  4. Emission                = SeasonN.json (Reload-Backend shape) + catalog
                              storefront entries for the game.
"""
import json, subprocess, sys, os, re, unicodedata

FAPI = '/tmp/fapi.json'
OUT_DIR = '/home/z/my-project/data/owen/battlepass'
CATALOG_OUT = '/home/z/my-project/data/owen/battlepass-catalog-21plus.json'
LLM_CACHE = '/tmp/bp_llm_cache'
os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(LLM_CACHE, exist_ok=True)

# internal season -> (marketing label, game version)
SEASONS = {
    21: ('Chapter 3 Season 3', 'v22.00'),
    22: ('Chapter 3 Season 4', 'v23.00'),
    23: ('Chapter 4 Season 1', 'v24.00'),
    24: ('Chapter 4 Season 2', 'v25.00'),
    25: ('Chapter 4 Season 3', 'v26.00'),
    26: ('Chapter 4 Season 4', 'v27.00'),
    27: ('Season OG (Chapter 4 Season 5)', 'v27.00'),
    28: ('Chapter 5 Season 1', 'v28.00'),
    29: ('Chapter 5 Season 2', 'v29.00'),
    30: ('Chapter 5 Season 3', 'v30.00'),
    31: ('Chapter 5 Season 4', 'v31.00'),
    32: ('Chapter 2 Remix', 'v32.00'),
    33: ('Chapter 6 Season 1', 'v33.00'),
    34: ('Chapter 6 Season 2', 'v34.00'),
    35: ('Chapter 6 Season 3', 'v35.00'),
    36: ('Chapter 6 Season 4', 'v36.00'),
    37: ('Chapter 7 Season 1', 'v37.00'),
    38: ('Chapter 7 Season 2', 'v38.00'),
    39: ('Chapter 7 Season 3', 'v39.00'),
    40: ('Chapter 7 Season 4', 'v40.00'),
    41: ('Chapter 7 Season 5', 'v41.00'),
    42: ('Chapter 7 Season 6', 'v42.00'),
}

# Real Epic offerIds where known (Arcane catalog); generated stable IDs
# for the rest (they match the catalog we emit).
REAL_OFFERS = {
    33: ('008A4B5B49BE59005267159A65DA1005', 'FC933E0F416ABD6372A8F584F9E7637D', '6ABDBABD42BCA1ED7EE2C7A5F8BCFEDD'),
}

# ---------------------------------------------------------------- fortnite-api
def load_index():
    d = json.load(open(FAPI))
    data = d.get('data') or []
    by_name = {}
    by_id = {}
    by_season = {}
    for it in data:
        intro = it.get('introduction') or {}
        bv = intro.get('backendValue')
        name = (it.get('name') or '').strip()
        if not name:
            continue
        rec = {
            'id': it['id'],
            'type': (it.get('type') or {}).get('backendValue', ''),
            'season': bv,
            'name': name,
        }
        by_name[name.lower()] = rec
        by_id[it['id'].lower()] = rec
        if bv:
            by_season.setdefault(bv, []).append(rec)
    return by_name, by_id, by_season

def norm(s):
    s = unicodedata.normalize('NFKD', s).encode('ascii', 'ignore').decode()
    return re.sub(r'[^a-z0-9]', '', s.lower())

# ------------------------------------------------------------------- GLM call
PROMPT = """You are a Fortnite data expert with precise knowledge of battle pass reward tables.

Fortnite {label} (internal Season {n}, game version {ver}) Battle Pass.

List EVERY battle pass reward with its tier number and track. Structure rules:
- The pass has 100 tiers. The paid ("Battle Pass") track has roughly 80-100 rewards.
- The free track has roughly 10-30 rewards.
- Outfits come with matching cosmetics sets (back blings, pickaxes, gliders, wraps, emotes, loading screens).
- Include V-Bucks tiers and any XP boost items.
- Use the exact in-game item names as shown in the locker.

Answer with STRICT JSON only, no markdown fences:
{{"rewards":[{{"name":"<item name>","type":"outfit|back bling|pickaxe|glider|wrap|emote|loading screen|spray|emoji|contrail|music|pet|banner|v-bucks|other","tier":<1-100>,"track":"paid|free"}}]}}"""

def ask_llm(n):
    cache = os.path.join(LLM_CACHE, f's{n}.json')
    if os.path.exists(cache) and os.path.getsize(cache) > 100:
        return open(cache).read()
    label, ver = SEASONS[n]
    prompt = PROMPT.format(label=label, n=n, ver=ver)
    out = f'/tmp/bp_llm_out_{n}.json'
    subprocess.run(
        ['z-ai', 'chat', '-p', prompt, '-t', '-o', out],
        capture_output=True, text=True, timeout=600)
    try:
        d = json.load(open(out))
        content = d['choices'][0]['message']['content']
    except Exception:
        content = ''
    open(cache, 'w').write(content)
    return content

def parse_llm(raw):
    txt = raw.strip()
    txt = re.sub(r'^```(json)?', '', txt).strip()
    txt = re.sub(r'```$', '', txt).strip()
    m = re.search(r'\{.*\}', txt, re.S)
    if not m:
        return []
    try:
        d = json.loads(m.group(0))
        return d.get('rewards', [])
    except Exception:
        return []

# ------------------------------------------------------------------ generation
def stable_guid(seed):
    import hashlib
    return hashlib.md5(seed.encode()).hexdigest().upper()

def generate(n, by_name, by_id, by_season):
    raw = ask_llm(n)
    rewards = parse_llm(raw)
    if not rewards:
        return None, {'season': n, 'error': 'llm parse failed', 'raw_len': len(raw)}

    verified, dropped, seen_ids = [], [], set()
    for r in rewards:
        name = r.get('name', '')
        track = 'free' if str(r.get('track', 'paid')).lower().startswith('f') else 'paid'
        tier = int(r.get('tier', 0) or 0)
        ttype = (r.get('type') or '').lower()

        if 'v-bucks' in name.lower() or 'vbucks' in ttype or 'v-bucks' in ttype:
            verified.append({'kind': 'mtx', 'tier': tier, 'track': track, 'amount': 100})
            continue
        if 'xp boost' in name.lower() or 'boost' in ttype:
            verified.append({'kind': 'boost', 'tier': max(tier, 1), 'track': track})
            continue

        rec = by_name.get(name.lower())
        if not rec:
            nn = norm(name)
            for k, v in by_name.items():
                if norm(k) == nn:
                    rec = v
                    break
        if not rec:
            rec = by_id.get(name.lower())
        if not rec or rec.get('season') != n:
            dropped.append(f"{name} (t{tier} {track})")
            continue
        if rec['id'].lower() in seen_ids:
            continue
        seen_ids.add(rec['id'].lower())
        verified.append({'kind': 'cosmetic', 'tier': tier, 'track': track,
                         'token': f"{rec['type']}:{rec['id']}"})

    if len([v for v in verified if v['kind'] == 'cosmetic']) < 15:
        return None, {'season': n, 'error': 'too few verified items',
                      'verified': len(verified), 'dropped': dropped[:10]}

    paid = [dict() for _ in range(100)]
    free = [dict() for _ in range(100)]

    cosmetic_paid = sorted([v for v in verified if v['kind'] == 'cosmetic' and v['track'] == 'paid'],
                           key=lambda v: (v['tier'] if v['tier'] > 0 else 999, v['token']))
    cosmetic_free = sorted([v for v in verified if v['kind'] == 'cosmetic' and v['track'] == 'free'],
                           key=lambda v: (v['tier'] if v['tier'] > 0 else 999, v['token']))

    def place(items, arr):
        cursor = 1
        for it in items:
            t = it['tier'] if 1 <= it['tier'] <= 100 else cursor
            t = max(t, cursor)
            if t > 100:
                t = min(cursor, 100)
            arr[t - 1][it['token']] = 1
            cursor = t + 1
        return cursor

    place(cosmetic_paid, paid)
    place(cosmetic_free, free)

    mtx_paid = [v for v in verified if v['kind'] == 'mtx' and v['track'] == 'paid']
    mtx_free = [v for v in verified if v['kind'] == 'mtx' and v['track'] == 'free']
    if len(mtx_paid) < 5:
        for t in (2, 12, 22, 32, 42, 52, 62, 72, 82, 92):
            paid[t - 1]['MtxGiveaway'] = 100
    else:
        for v in mtx_paid:
            t = v['tier'] if 1 <= v['tier'] <= 100 else 2
            paid[t - 1]['MtxGiveaway'] = v['amount']
    if len(mtx_free) < 2:
        for t in (6, 26, 46):
            free[t - 1]['MtxGiveaway'] = 100
    else:
        for v in mtx_free:
            t = v['tier'] if 1 <= v['tier'] <= 100 else 6
            free[t - 1]['MtxGiveaway'] = v['amount']

    paid[0]['Token:athenaseasonxpboost'] = 50
    paid[0]['Token:athenaseasonfriendxpboost'] = 60

    def trim(arr):
        while arr and not arr[-1]:
            arr.pop()
        return arr or [{}]

    bundle, pas, tier = REAL_OFFERS.get(n, (
        stable_guid(f'owen-s{n}-bundle'),
        stable_guid(f'owen-s{n}-pass'),
        stable_guid(f'owen-s{n}-tier'),
    ))

    doc = {
        'battleBundleOfferId': bundle,
        'battlePassOfferId': pas,
        'tierOfferId': tier,
        'paidRewards': trim(paid),
        'freeRewards': trim(free),
    }
    stats = {
        'season': n,
        'paid_tiers': len(doc['paidRewards']),
        'free_tiers': len(doc['freeRewards']),
        'cosmetics_paid': len(cosmetic_paid),
        'cosmetics_free': len(cosmetic_free),
        'dropped': dropped[:12],
        'season_pool': len(by_season.get(n, [])),
    }
    return doc, stats

def catalog_entry(n, offer_id, dev, title, price, regular=None, sale=None):
    return {
        'offerId': offer_id,
        'devName': f'BR.Season{n}.{dev}.01',
        'offerType': 'StaticPrice',
        'title': title,
        'description': '',
        'itemGrants': [],
        'categories': [],
        'catalogGroup': '',
        'catalogGroupPriority': 0,
        'sortPriority': 0,
        'fulfillmentIds': [],
        'dailyLimit': -1,
        'weeklyLimit': -1,
        'monthlyLimit': -1,
        'purchaseLimit': -1,
        'displayAssetPath': '',
        'filterWeight': 0,
        'appStoreId': [],
        'requirements': [],
        'metaInfo': [],
        'catalogTrackingId': '',
        'presentationBundleId': None,
        'prices': [{
            'currencyType': 'MtxCurrency',
            'currencySubType': '',
            'regularPrice': regular if regular is not None else price,
            'dynamicRegularPrice': -1,
            'finalPrice': price,
            **({'saleType': sale, 'saleExpiration': '9999-12-31T23:59:59.999Z'} if sale else {}),
            'basePrice': price,
        }],
    }

def main():
    by_name, by_id, by_season = load_index()
    all_stats, catalog = [], []
    for n in sorted(SEASONS):
        doc, stats = generate(n, by_name, by_id, by_season)
        if doc is None:
            print(f"[S{n}] SKIP: {stats}")
            all_stats.append(stats)
            continue
        path = os.path.join(OUT_DIR, f'Season{n}.json')
        json.dump(doc, open(path, 'w'), indent=2)
        print(f"[S{n}] OK paid={stats['paid_tiers']}T/{stats['cosmetics_paid']}c "
              f"free={stats['free_tiers']}T/{stats['cosmetics_free']}c "
              f"pool={stats['season_pool']} dropped={len(stats['dropped'])}")
        all_stats.append(stats)
        catalog.append(catalog_entry(n, doc['battleBundleOfferId'], 'BattleBundle',
                                     {'en': 'Battle Bundle'}, 2800, regular=4700, sale='Strikethrough'))
        catalog.append(catalog_entry(n, doc['battlePassOfferId'], 'BattlePass',
                                     {'en': 'Battle Pass'}, 950))
        catalog.append(catalog_entry(n, doc['tierOfferId'], 'SingleTier',
                                     {'en': 'Battle Pass Tier'}, 150))
    json.dump({'storefronts': [{'name': 'BRSeason21Plus', 'catalogEntries': catalog}]},
              open(CATALOG_OUT, 'w'), indent=2)
    json.dump(all_stats, open('/tmp/bp_stats.json', 'w'), indent=1)
    print(f"\ncatalog entries: {len(catalog)} -> {CATALOG_OUT}")

if __name__ == '__main__':
    main()
