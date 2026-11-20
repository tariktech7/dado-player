// Rasterize the canonical SVG for each platform; never redraw raster logos.
import {createRequire} from 'node:module';
import {existsSync} from 'node:fs';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const root=fileURLToPath(new URL('../',import.meta.url));
const require=createRequire(new URL('../SmartTV/client/package.json',import.meta.url));
const {chromium}=require('@playwright/test');
const svg=await readFile(path.join(root,'assets/d7.svg'),'utf8');
const darkSvg=await readFile(path.join(root,'assets/d7-dark.svg'),'utf8');
const browser=await chromium.launch();
const page=await browser.newPage({deviceScaleFactor:1});
async function render(file,width,height,markup){
 await page.setViewportSize({width,height});
 await page.setContent(`<html><style>html,body{margin:0;width:100%;height:100%;background:transparent}svg{display:block;width:100%;height:100%}</style>${markup}</html>`);
 await page.screenshot({path:path.join(root,file),omitBackground:true});
 console.log('Rendered',file,width,height);
}
try{
 if(!process.argv.includes('--social-only')){
 await render('assets/icon.png',512,512,svg);
 await render('assets/icon-dark.png',512,512,darkSvg);
 for(const [name,size] of [['AppIcon60@2x',120],['AppIcon60@3x',180],['AppIcon76',76],['AppIcon76@2x',152],['AppIcon83.5@2x',167]])
  await render('ios/Resources/'+name+'.png',size,size,svg);
 await render('android/app/src/main/res/mipmap-mdpi/ic_launcher.png',48,48,svg);
 const inner=svg.slice(svg.indexOf('>')+1,svg.lastIndexOf('</svg>'));
 await render('android/app/src/main/res/drawable-nodpi/d7_foreground.png',324,324,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192"><g transform="translate(32 32)">${inner}</g></svg>`);
 await render('android/app/src/main/res/drawable-nodpi/d7_banner.png',320,180,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 180"><rect width="320" height="180" fill="#12283f"/><g transform="translate(28 26)">${inner}</g><text x="172" y="79" fill="#edf4ff" font-family="Arial" font-size="22" font-weight="bold">Dado</text><text x="172" y="108" fill="#91c9ff" font-family="Arial" font-size="22">Player</text></svg>`);
 if(existsSync(path.join(root,'website/index.html')))await writeFile(path.join(root,'website/assets/icon.png'),await readFile(path.join(root,'assets/icon.png')));
 }
 const socialInner=svg.slice(svg.indexOf('>')+1,svg.lastIndexOf('</svg>'));
 if(existsSync(path.join(root,'website/index.html')))await render('website/assets/images/og-image.png',1200,630,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 630"><rect width="1200" height="630" fill="#19334e"/><rect x="45" y="45" width="1110" height="540" rx="40" fill="#224462"/><g transform="translate(100 180) scale(2)">${socialInner}</g><text x="420" y="280" fill="#edf4ff" font-family="Arial" font-size="76" font-weight="bold">Dado Player</text><text x="424" y="350" fill="#bfd0e2" font-family="Arial" font-size="35">Dein Programm. Dein Player.</text><text x="424" y="415" fill="#91c9ff" font-family="Arial" font-size="25">Windows · Android · iOS · Smart TV</text></svg>`);
}finally{await browser.close();}
