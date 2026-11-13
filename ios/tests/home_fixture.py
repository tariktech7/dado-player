"""Populate only the throwaway simulator container, never a user's profile."""
import json
import pathlib
import sys

container = pathlib.Path(sys.argv[1])
config = container / 'Library/Application Support/Library/Application Support/com.Tarik.Dado-Player'
config.mkdir(parents=True, exist_ok=True)
playlist = container / 'Documents/home.m3u'
playlist.write_text('#EXTM3U\n' + ''.join(
    f'#EXTINF:-1 tvg-id="qa-{i}" group-title="Test",Dado Test {i}\ndado:demo\n'
    for i in range(1, 13)))
profile = dict(id='qa', name='Simulator', kind='M3u', host='', username='', password='',
               m3u_url='', m3u_file=str(playlist), last_channel=None)
for name, value in {
    'profiles.json': [profile],
    'settings.json': dict(default_profile='qa', auto_play_last=False, theme='Dark', settings_version=4),
    'favorites.json': ['live:1', 'live:2'],
}.items():
    (config / name).write_text(json.dumps(value))
