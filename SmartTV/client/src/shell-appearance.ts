import appearance from '../../../shared/shell-appearance.json' with {type:'json'};
import palette from '../../../shared/normal-palette.json' with {type:'json'};
import classicTilePalette from '../../../shared/classic-tile-palette.json' with {type:'json'};
import ambient from '../../../shared/ambient-palette.json' with {type:'json'};
export function installShellAppearance() {
  const style=document.documentElement.style;
  for (const [theme, lights] of Object.entries(ambient)) {
    style.setProperty(`--ambient-${theme}-near`, lights.near);
    style.setProperty(`--ambient-${theme}-far`, lights.far);
  }
  for(const [key,value] of Object.entries(palette))style.setProperty('--normal-'+key.replace(/[A-Z]/g,c=>'-'+c.toLowerCase()),value);
  for (const [tile, colors] of Object.entries(classicTilePalette)) {
    for (const [position, color] of Object.entries(colors)) style.setProperty(`--classic-${tile}-${position}`, color);
  }
  for(const [key,value] of Object.entries({
    '--navigation-expanded':appearance.navigationExpanded+'px',
    '--navigation-edge':appearance.navigationEdge+'px',
    '--live-preview-edge':appearance.livePreviewEdge+'px',
    '--navigation-backdrop-scale':String((appearance.navigationCollapsed+appearance.navigationEdge+10)/(appearance.navigationExpanded+appearance.navigationEdge+10)),
    '--refresh-spin-duration':appearance.refreshSpinMs+'ms',
    '--navigation-collapsed':appearance.navigationCollapsed+'px',
    '--category-width':appearance.categoryWidth+'px',
    '--live-preview-width':appearance.livePreviewWidth+'px',
    '--classic-category-width':appearance.classicCategoryShare*100+'%',
    '--classic-poster-row-height':appearance.classicPosterRowHeight+'px',
    '--classic-live-category-width':appearance.classicLiveCategoryShare*100+'%',
    '--classic-live-preview-width':appearance.classicLivePreviewShare*100+'%',
    '--classic-header-height':appearance.classicHeaderHeight+'px',
    '--control-radius':appearance.controlRadius+'px',
    '--settings-panel-radius':appearance.settingsPanelRadius+'px',
    '--settings-panel-width':appearance.settingsPanelWidth+'px',
    '--classic-dashboard-height':appearance.classicDashboardHeight+'px',
    '--classic-dashboard-icon-size':appearance.classicDashboardIconSize+'px',
    '--classic-dashboard-title-size':appearance.classicDashboardTitleSize+'px',
    '--classic-dashboard-gap':appearance.classicDashboardGap+'px',
    '--classic-action-height':appearance.classicActionHeight+'px',
    '--classic-dashboard-live-share':appearance.classicDashboardLiveShare*100+'fr',
    '--classic-dashboard-other-share':(1-appearance.classicDashboardLiveShare)*50+'fr',
    '--classic-category-font':appearance.classicCategoryFont+'px',
    '--classic-channel-font':appearance.classicChannelFont+'px',
    '--classic-gap':appearance.classicGap+'px',
    '--navigation-font':appearance.navigationFont+'px',
    '--player-top-height':appearance.playerTopHeight+'px',
    '--player-controls-height':appearance.playerControlsHeight+'px',
    '--player-top-inset-x':appearance.playerTopInsetX+'px',
    '--player-top-inset-y':appearance.playerTopInsetY+'px',
    '--player-controls-inset-x':appearance.playerControlsInsetX+'px',
    '--player-controls-inset-y':appearance.playerControlsInsetY+'px',
    '--player-panel-color':appearance.playerPanelColor,
    '--live-scrim-color':appearance.liveScrimColor,
    '--live-scrim-height':appearance.liveScrimHeight+'px',
    '--live-controls-height':appearance.liveControlsHeight+'px',
    '--live-logo-width':appearance.liveLogoWidth+'px',
    '--live-logo-height':appearance.liveLogoHeight+'px',
    '--live-number-font':appearance.liveNumberFont+'px',
    '--live-title-font':appearance.liveTitleFont+'px',
    '--live-programme-font':appearance.liveProgrammeFont+'px',
    '--live-shortcut-font':appearance.liveShortcutFont+'px',
    '--live-shortcut-dot':appearance.liveShortcutDot+'px',
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
