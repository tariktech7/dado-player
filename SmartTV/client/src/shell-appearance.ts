import appearance from '../../../shared/shell-appearance.json' with {type:'json'};
import palette from '../../../shared/normal-palette.json' with {type:'json'};
export function installShellAppearance() {
  const style=document.documentElement.style;
  for(const [key,value] of Object.entries(palette))style.setProperty('--normal-'+key.replace(/[A-Z]/g,c=>'-'+c.toLowerCase()),value);
  for(const [key,value] of Object.entries({
    '--navigation-expanded':appearance.navigationExpanded+'px',
    '--navigation-collapsed':appearance.navigationCollapsed+'px',
    '--category-width':appearance.categoryWidth+'px',
    '--live-preview-width':appearance.livePreviewWidth+'px',
    '--catalog-preview-width':appearance.catalogPreviewWidth+'px',
    '--navigation-font':appearance.navigationFont+'px',
    '--player-top-height':appearance.playerTopHeight+'px',
    '--player-controls-height':appearance.playerControlsHeight+'px',
    '--player-corner-radius':appearance.playerCornerRadius+'px',
  }))style.setProperty(key,value);
}
export default appearance;
