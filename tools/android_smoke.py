#!/usr/bin/env python3
"""Install the public APK and retain startup evidence from an isolated emulator."""
from pathlib import Path
import json, subprocess, time
app = 'dev.taldoshawn.pojav'
evidence = Path('evidence')
apk = next(Path('apk').glob('*.apk'))
def adb(*args, check=True):
    return subprocess.run(['adb', *args], check=check, capture_output=True)
failed = None
installed = False
try:
    install = adb('install', '-r', '-g', str(apk), check=False)
    (evidence/'install.txt').write_bytes(install.stdout + install.stderr)
    if install.returncode or b'Success' not in install.stdout:
        raise RuntimeError('APK installation failed; see install.txt')
    installed = True
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
except Exception as error:
    failed = str(error)
finally:
    (evidence/'logcat.txt').write_bytes(adb('logcat', '-d', check=False).stdout)
    (evidence/'startup.png').write_bytes(adb('exec-out', 'screencap', '-p', check=False).stdout)
    (evidence/'RESULT.json').write_text(json.dumps({'package':app,'installed':installed,
        'startup_passed':failed is None,'failure':failed,'test':'x86_64 emulator API 35 startup only',
        'world_or_multiplayer_tested':False,'fps_benchmark':False},indent=2)+'\n')
if failed:
    raise SystemExit(failed)
