import json
import os
import traceback

import requests


for i in range(3):
    try:
        url = 'https://api.github.com/repos/bugdone/demoinfogo-linux/releases'
        auth = None
        if os.environ.get('GITHUB_TOKEN'):
            auth = (os.environ['GITHUB_USER'], os.environ['GITHUB_TOKEN'])
        demoinfogo = requests.get(url, auth=auth)
        print demoinfogo.status_code, demoinfogo.headers, demoinfogo.content
        assets = json.loads(demoinfogo.content)[0]['assets']
        for url in (assets[0]['browser_download_url'], assets[1]['browser_download_url']):
            os.system('wget ' + url)
        break
    except Exception as e:
        traceback.print_exc()
