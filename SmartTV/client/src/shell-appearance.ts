import appearance from '../../../shared/shell-appearance.json' with {type:'json'};
import palette from '../../../shared/normal-palette.json' with {type:'json'};
export function installShellAppearance() {
  const style=document.documentElement.style;
  for(const [key,value] of Object.entries(palette))style.setProperty('--normal-'+key.replace(/[A-Z]/g,c=>'-'+c.toLowerCase()),value);
  for(const [key,value] of Object.entries({
    '--navigation-expanded':appearance.navigationExpanded+'px',
    '--navigation-edge':appearance.navigationEdge+'px',
    '--live-preview-edge':appearance.livePreviewEdge+'px',
    '--navigation-backdrop-scale':String((appearance.navigationCollapsed+appearance.navigationEdge+10)/(appearance.navigationExpanded+appearance.navigationEdge+10)),
    '--refresh-spin-duration':appearance.refreshSpinMs+'ms',
    '--navigation-collapsed':appearance.navigationCollapsed+'px',
    '--category-width':appearance.categoryWidth+'px',
    '--live-preview-width':appearance.livePreviewWidth+'px',
    '--catalog-preview-width':appearance.catalogPreviewWidth+'px',
    '--navigation-font':appearance.navigationFont+'px',
    '--player-top-height':appearance.playerTopHeight+'px',
    '--player-controls-height':appearance.playerControlsHeight+'px',
    '--player-top-inset-x':appearance.playerTopInsetX+'px',
    '--player-top-inset-y':appearance.playerTopInsetY+'px',
    '--player-controls-inset-x':appearance.playerControlsInsetX+'px',
    '--player-controls-inset-y':appearance.playerControlsInsetY+'px',
    '--player-panel-color':appearance.playerPanelColor,
    '--player-button-color':appearance.playerButtonColor,
    '--player-primary-color':appearance.playerPrimaryColor,
    '--player-focus-color':appearance.playerFocusColor,
    '--player-text-color':appearance.playerTextColor,
    '--player-muted-color':appearance.playerMutedColor,
    '--player-top-max-width':appearance.playerTopMaxWidth+'px',
    '--player-controls-max-width':appearance.playerControlsMaxWidth+'px',
    '--player-border-color':appearance.playerBorderColor,
    '--player-button-height':appearance.playerButtonHeight+'px',
    '--player-button-font':appearance.playerButtonFont+'px',
    '--player-popup-padding':appearance.playerPopupPadding+'px',
    '--player-popup-gap':appearance.playerPopupGap+'px',
    '--player-popup-offset':appearance.playerPopupOffset+'px',
    '--player-corner-radius':appearance.playerCornerRadius+'px',
  }))style.setProperty(key,value);
}
export default appearance;
