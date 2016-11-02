# environment paths demoinfogo_bin_linux and demoinfogo_bin_win
# should point to directories where the demoinfogo binaries are
import os
import os.path
import shutil


os.system('lein clean')
os.system('LEIN_SNAPSHOTS_IN_RELEASE=1 lein uberjar')
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

    def write_launcher(filename, content):
        with open(path + '/' + filename, 'w') as f:
            f.write(content % jar)

    if os_name == 'win':
        write_launcher('headshotbox.bat', 'start javaw -jar %s --port 4000 --systray')
        write_launcher('headshotbox_console.bat', 'java -jar %s --port 4000')
    else:
        write_launcher('headshotbox.sh', 'java -jar %s --port 4000')
    if os_name == 'linux':
        os.system('chmod +x %s/headshotbox.sh' % path)

    shutil.make_archive(dir_name, 'zip', dir_name)
