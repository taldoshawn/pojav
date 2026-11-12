#!/usr/bin/env python3
"""Install the public APK and retain startup evidence from an isolated emulator."""
from pathlib import Path
import json, subprocess, time, os
app = 'dev.taldoshawn.pojav'
evidence = Path('evidence')
apk = next(Path('apk').glob('*.apk'))
def adb(*args, check=True):
    return subprocess.run(['adb', *args], check=check, capture_output=True, timeout=120)
failed = None
installed = False
try:
    install = adb('install', '-r', '-g', str(apk), check=False)
    (evidence/'install.txt').write_bytes(install.stdout + install.stderr)
    if install.returncode or b'Success' not in install.stdout:
        raise RuntimeError('APK installation failed; see install.txt')
    installed = True
    server = os.environ.get('TEST_SERVER', 'none')
    if server != 'none':
        if server != 'mush':
            raise ValueError('Unsupported public server probe')
        # Seed only public launcher preferences in this disposable emulator.
        # This does not create a server account, send chat, or alter authentication.
        adb('root')
        adb('wait-for-device')
        data = '/data/user/0/' + app
        uid = adb('shell', 'stat', '-c', '%u', data).stdout.decode().strip()
        if not uid.isdigit():
            raise RuntimeError('Could not resolve the installed app UID')
        adb('shell', 'mkdir', '-p', data + '/files')
        adb('shell', 'chown', uid + ':' + uid, data + '/files')
        fixture = Path('launcher-test-profile.json')
        fixture.write_text(json.dumps({'name': 'PRust' + os.environ.get('GITHUB_RUN_ID', 'Probe')[-8:],
                                      'server': 'jogar.mush.com.br'}))
        adb('push', str(fixture), data + '/files/android-launcher.json')
        adb('shell', 'chown', uid + ':' + uid, data + '/files/android-launcher.json')
        adb('shell', 'chmod', '600', data + '/files/android-launcher.json')
        adb('unroot')
        adb('wait-for-device')
    adb('logcat', '-c')
    started = adb('shell', 'am', 'start', '-W', '-n', app+'/com.google.androidgamesdk.GameActivity', check=False)
    (evidence/'start.txt').write_bytes(started.stdout + started.stderr)
    if started.returncode or b'Error' in started.stdout + started.stderr:
        raise RuntimeError('Activity start failed; see start.txt')
    for _ in range(12):
        time.sleep(5)
        pid = adb('shell', 'pidof', app, check=False).stdout.decode().strip()
        if not pid:
            raise RuntimeError('App process exited during startup')
    activity = adb('shell', 'dumpsys', 'activity', 'activities').stdout
    (evidence/'activities.txt').write_bytes(activity)
    if not any(app.encode() in line and (b'ResumedActivity' in line or b'topResumedActivity' in line) for line in activity.splitlines()):
        raise RuntimeError('App activity is not resumed')
    app_log = adb('logcat', '-d', '--pid', pid).stdout
    if any(marker in app_log for marker in [b'FATAL EXCEPTION', b'Fatal signal', b'panicked at']):
        raise RuntimeError('App crash marker in logcat')
    (evidence/'launcher.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
    if os.environ.get('PROBE_GAME') == 'true':
        # Pixel 2 landscape, fixed profile above. This explores UI and records
        # screenshots for inspection; it does not assert successful gameplay.
        adb('shell', 'input', 'tap', '1300', '190')
        for i in range(6):
            time.sleep(30)
            (evidence/f'download-{i}.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
        adb('shell', 'input', 'tap', '1300', '190')
        for i in range(4):
            time.sleep(30)
            (evidence/f'game-{i}.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
            if not adb('shell', 'pidof', app, check=False).stdout.strip():
                raise RuntimeError('App exited during resource/game probe')
        if os.environ.get('PROBE_WORLD') == 'true':
            # Coordinates verified against this Pixel 2 profile's screenshots
            # and the title/create-world layout (GUI scale 4).
            def tap_capture(x, y, name, delay=5):
                adb('shell', 'input', 'tap', str(x), str(y))
                time.sleep(delay)
                (evidence/(name+'.png')).write_bytes(adb('exec-out', 'screencap', '-p').stdout)
            tap_capture(896, 1010, 'main-menu')  # onboarding Continue
            tap_capture(896, 500, 'create-world', 30)  # Singleplayer
            tap_capture(896, 45, 'world-tab')
            tap_capture(576, 200, 'flat-preset')
            tap_capture(576, 1010, 'world-loading', 30)
            for i in range(6):
                time.sleep(30)
                (evidence/f'world-{i}.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
                if not adb('shell', 'pidof', app, check=False).stdout.strip():
                    raise RuntimeError('App exited during world probe')
            adb('shell', 'input', 'swipe', '1100', '540', '1300', '700', '800')
            time.sleep(10)
            (evidence/'world-after-touch.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
except Exception as error:
    failed = str(error)
finally:
    (evidence/'logcat.txt').write_bytes(adb('logcat', '-d', check=False).stdout)
    (evidence/'startup.png').write_bytes(adb('exec-out', 'screencap', '-p', check=False).stdout)
    (evidence/'RESULT.json').write_text(json.dumps({'package':app,'installed':installed,
        'startup_passed':failed is None,'failure':failed,'test':'x86_64 emulator API 35 startup only',
        'world_or_multiplayer_tested':False,'fps_benchmark':False,
        'public_server_probe':os.environ.get('TEST_SERVER', 'none'),
        'screenshots_require_manual_gameplay_review':True},indent=2)+'\n')
if failed:
    raise SystemExit(failed)
