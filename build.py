# environment paths demoinfogo_bin__linux and demoinfogo_bin_win
# should point to directories where the demoinfogo binaries are
import os
import os.path
import shutil


os.system('lein clean')
os.system('lein uberjar')
jar = [f for f in os.listdir('./target') if f.endswith('-standalone.jar')][0]
version = jar.split('-')[1]

for os_name in ['linux', 'win']:
    dir_name = 'headshotbox-%s-%s' % (version, os_name)
    path = dir_name + '/' + dir_name
    demoinfogo = os.environ['demoinfogo_bin_' + os_name]
    if os.path.exists(dir_name):
        shutil.rmtree(dir_name)
    os.mkdir(dir_name)
    shutil.copytree(demoinfogo, path)
    shutil.copy('target/' + jar, path)
    with open(path + '/headshotbox.' + ('bat' if os_name == 'win' else 'sh'), 'w') as f:
        f.write('java -jar %s 4000' % jar)
    if os_name == 'linux':
        os.system('chmod +x %s/headshotbox.sh' % path)

    shutil.make_archive(dir_name, 'zip', dir_name)
