import urllib
import json
import os

demoinfogo = urllib.urlopen('https://api.github.com/repos/bugdone/demoinfogo-linux/releases').read()
assets = json.loads(demoinfogo)[0]['assets']
for url in (assets[0]['browser_download_url'], assets[1]['browser_download_url']):
    os.system('wget ' + url)
