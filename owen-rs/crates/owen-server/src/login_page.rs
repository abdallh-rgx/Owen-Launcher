//! Embedded login page (professional, English, no emojis).
//!
//! Served at `/login` — the target the Kairos WebView gets redirected to by
//! the Owen redirecter (no smali edits, the redirect happens via ART hooks
//! inside the game process). Both `http://` and `https://` origins work.

pub const LOGIN_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Owen - Sign in</title>
<style>
:root{--bg:#0b0d10;--card:#14181e;--line:#232a34;--text:#e8ecf1;--muted:#8b95a3;--accent:#10b981;--accent2:#0d9488}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;background:var(--bg);color:var(--text);min-height:100vh;display:flex;align-items:center;justify-content:center;padding:24px}
.card{width:100%;max-width:400px;background:var(--card);border:1px solid var(--line);border-radius:14px;padding:32px}
.brand{display:flex;align-items:center;gap:10px;margin-bottom:28px}
.mark{width:34px;height:34px;border-radius:8px;background:linear-gradient(135deg,var(--accent),var(--accent2));display:flex;align-items:center;justify-content:center;font-weight:700;color:#04110c;font-size:16px}
.brand h1{font-size:17px;font-weight:600;letter-spacing:.2px}
.brand span{color:var(--muted);font-size:12px;display:block;font-weight:400}
label{display:block;font-size:12px;color:var(--muted);margin:18px 0 6px;text-transform:uppercase;letter-spacing:.6px}
input{width:100%;padding:12px 14px;border-radius:9px;border:1px solid var(--line);background:#0f1318;color:var(--text);font-size:15px;outline:none;transition:border .15s}
input:focus{border-color:var(--accent)}
button{width:100%;margin-top:24px;padding:13px;border:0;border-radius:9px;background:var(--accent);color:#04110c;font-size:15px;font-weight:600;cursor:pointer;transition:background .15s}
button:hover{background:#34d399}
button:disabled{opacity:.6;cursor:default}
.note{margin-top:16px;font-size:12px;color:var(--muted);line-height:1.6}
.err{margin-top:14px;padding:10px 12px;border-radius:8px;background:rgba(239,68,68,.08);border:1px solid rgba(239,68,68,.3);color:#fca5a5;font-size:13px;display:none}
.ok{margin-top:14px;padding:10px 12px;border-radius:8px;background:rgba(16,185,129,.08);border:1px solid rgba(16,185,129,.3);color:#6ee7b7;font-size:13px;display:none}
.foot{margin-top:22px;font-size:11px;color:var(--muted);text-align:center}
</style>
</head>
<body>
<main class="card">
  <div class="brand">
    <div class="mark">O</div>
    <div><h1>Owen Backend</h1><span>Private server authentication</span></div>
  </div>
  <div class="err" id="err"></div>
  <div class="ok" id="ok"></div>
  <label for="name">Display name</label>
  <input id="name" autocomplete="off" spellcheck="false" placeholder="Your in-game name" maxlength="32">
  <button id="go" onclick="authorize()">Authorize</button>
  <p class="note">Authorization completes instantly and returns you to the game. The name you enter becomes your account identity on this server.</p>
  <p class="foot" id="origin"></p>
</main>
<script>
const qs=new URLSearchParams(location.search);
if(qs.get('name'))document.getElementById('name').value=qs.get('name');
document.getElementById('origin').textContent=location.origin;
async function authorize(){
  const err=document.getElementById('err'),ok=document.getElementById('ok'),btn=document.getElementById('go');
  const name=document.getElementById('name').value.trim();
  if(!name){err.textContent='Enter a display name first.';err.style.display='block';return}
  err.style.display='none';ok.style.display='none';btn.disabled=true;btn.textContent='Authorizing...';
  try{
    const body=new URLSearchParams({grant_type:'password',username:name,password:'owen',token_type:'eg1'});
    const t=await fetch('/account/api/oauth/token',{method:'POST',headers:{'Content-Type':'application/x-www-form-urlencoded','Authorization':'Basic b3dlbjpvd2Vu'},body});
    const tj=await t.json();
    if(!t.ok)throw new Error(tj.error_description||'Token request failed');
    const e=await fetch('/account/api/oauth/exchange',{headers:{'Authorization':'Bearer '+tj.access_token}});
    const ej=await e.json();
    if(!e.ok)throw new Error('Exchange failed');
    ok.textContent='Authorized. Returning to the game...';ok.style.display='block';
    setTimeout(()=>{location.href='com.epicgames.fortnite://authorize/?code='+encodeURIComponent(ej.code);},400);
  }catch(ex){err.textContent=ex.message||'Authorization failed';err.style.display='block';btn.disabled=false;btn.textContent='Authorize';}
}
document.addEventListener('keydown',e=>{if(e.key==='Enter')authorize()});
</script>
</body>
</html>"#;
