import urllib
import json
import os
import traceback


for i in range(3):
    try:
        demoinfogo = urllib.urlopen('https://api.github.com/repos/bugdone/demoinfogo-linux/releases').read()
        print demoinfogo
        assets = json.loads(demoinfogo)[0]['assets']
        for url in (assets[0]['browser_download_url'], assets[1]['browser_download_url']):
            os.system('wget ' + url)
        break
    except Exception as e:
        traceback.print_exc()
