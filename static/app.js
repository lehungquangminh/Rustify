const $ = (s)=>document.querySelector(s);
const form = $('#shorten-form');
const urlIn = $('#url');
const aliasIn = $('#alias');
const result = $('#result');
const shortLink = $('#short-link');
const qr = $('#qr');
const statsCard = $('#stats');
const statsJson = $('#stats-json');

form.addEventListener('submit', async (e)=>{
  e.preventDefault();
  result.classList.add('hidden');
  statsCard.classList.add('hidden');
  try{
    const payload = { url: urlIn.value };
    if(aliasIn.value.trim()) payload.alias = aliasIn.value.trim();
    const res = await fetch('/shorten', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(payload)
    });
    if(!res.ok) throw new Error(await res.text());
    const data = await res.json();
    shortLink.href = data.short_url;
    shortLink.textContent = data.short_url;
    qr.src = data.short_url; 
    qr.src = data.short_url;
    qr.src = data.short_url;
    qr.src = data.short_url + '';
    qr.src = data.short_url + '';
    const qrRes = await fetch(data.short_url, { headers: { 'accept': 'image/png' } });
    const blob = await qrRes.blob();
    qr.src = URL.createObjectURL(blob);
    result.classList.remove('hidden');
    const alias = new URL(data.short_url).pathname.replace(/^\//,'');
    const s = await fetch('/stats/' + alias);
    if(s.ok){ statsJson.textContent = JSON.stringify(await s.json(), null, 2); statsCard.classList.remove('hidden'); }
  }catch(err){
    alert('Lỗi: ' + err.message);
  }
});
