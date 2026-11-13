#!/usr/bin/env python3
"""Install the public APK and retain startup evidence from an isolated emulator."""
from pathlib import Path
import json, subprocess, time, os
app = 'dev.taldoshawn.pojav'
evidence = Path('evidence')
apk = next(Path('apk').glob('*.apk'))
def adb(*args, check=True):
    return subprocess.run(['adb', *args], check=check, capture_output=True, timeout=120)
def adb_privilege(root):
    # adbd may disconnect before acknowledging a successful restart. Verify
    # the new daemon's UID instead of treating its transport exit as failure.
    command = 'root' if root else 'unroot'
    result = adb(command, check=False)
    adb('wait-for-device')
    uid = adb('shell', 'id', '-u').stdout.strip()
    if (uid == b'0') != root:
        raise RuntimeError('adbd privilege transition failed: ' +
                           (result.stdout + result.stderr).decode(errors='replace'))

failed = None
startup_passed = False
profiles_verified = False
legacy_verified = False
legacy_fixture = None
legacy_peer = None
peer_log = None
pvp_attack_verified = False
installed = False
try:
    install = adb('install', '-r', '-g', str(apk), check=False)
    (evidence/'install.txt').write_bytes(install.stdout + install.stderr)
    if install.returncode or b'Success' not in install.stdout:
        raise RuntimeError('APK installation failed; see install.txt')
    installed = True
    server = os.environ.get('TEST_SERVER', 'none')
    if server != 'none':
        if server not in ('mush', 'legacy18'):
            raise ValueError('Unsupported public server probe')
        if server == 'legacy18':
            from legacy_test_server import LegacyServer
            legacy_fixture = LegacyServer(evidence)
            legacy_fixture.start()
        # Seed only public launcher preferences in this disposable emulator.
        # This does not create a server account, send chat, or alter authentication.
        adb_privilege(True)
        data = '/data/user/0/' + app
        uid = adb('shell', 'stat', '-c', '%u', data).stdout.decode().strip()
        if not uid.isdigit():
            raise RuntimeError('Could not resolve the installed app UID')
        adb('shell', 'mkdir', '-p', data + '/files')
        adb('shell', 'chown', uid + ':' + uid, data + '/files')
        fixture = Path('launcher-test-profile.json')
        fixture.write_text(json.dumps({'name': 'LegacyAndroid' if server == 'legacy18' else 'PRust' + os.environ.get('GITHUB_RUN_ID', 'Probe')[-8:],
                                      'server': '10.0.2.2:25565' if server == 'legacy18' else 'jogar.mush.com.br',
                                      'legacy18': server == 'legacy18'}))
        adb('push', str(fixture), data + '/files/android-launcher.json')
        adb('shell', 'chown', uid + ':' + uid, data + '/files/android-launcher.json')
        adb('shell', 'chmod', '600', data + '/files/android-launcher.json')
        adb_privilege(False)
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
    startup_passed = True
    if os.environ.get('PROBE_PROFILES') == 'true':
        def profile_tap(x, y):
            adb('shell', 'input', 'tap', str(x), str(y))
            time.sleep(2)
        def profile_capture(name):
            (evidence/(name+'.png')).write_bytes(adb('exec-out', 'screencap', '-p').stdout)
        def enter_local(name):
            profile_tap(600, 500)  # Offline provider
            profile_capture('profile-provider-' + name)
            profile_tap(900, 250)  # Player name field
            adb('shell', 'input', 'keyevent', '123', *(['67'] * 16))
            adb('shell', 'input', 'text', name)
            adb('shell', 'input', 'keyevent', '4')
            time.sleep(2)
            profile_tap(900, 980)  # Finish editing, then save/use
            profile_tap(900, 980)
            profile_capture('profile-saved-' + name)
        profile_tap(1650, 60)
        profile_capture('profile-providers')
        enter_local('PojavOne')
        profile_tap(1650, 60)
        profile_tap(740, 980)  # Add account
        enter_local('PojavTwo')
        profile_tap(1650, 60)
        profile_capture('profile-list')
        profile_tap(900, 375)  # First saved local profile
        adb('shell', 'am', 'force-stop', app)
        adb('shell', 'am', 'start', '-W', '-n', app+'/com.google.androidgamesdk.GameActivity')
        time.sleep(10)
        profile_capture('profile-after-restart')
        adb_privilege(True)
        files = '/data/user/0/' + app + '/files/'
        accounts = json.loads(adb('shell', 'cat', files+'accounts.json').stdout)
        config = json.loads(adb('shell', 'cat', files+'android-launcher.json').stdout)
        report = {'saved_names': accounts.get('offline'),
                  'selected_local': accounts.get('offline_selected'),
                  'selected_online': accounts.get('selected'), 'launcher_name': config.get('name')}
        (evidence/'local-profiles.json').write_text(json.dumps(report, indent=2)+'\n')
        adb_privilege(False)
        if report != {'saved_names':['PojavOne','PojavTwo'], 'selected_local':'PojavOne',
                      'selected_online':None, 'launcher_name':'PojavOne'}:
            raise RuntimeError('Local profile creation/switch/restart failed; see screenshots and local-profiles.json')
        profiles_verified = True
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
        if legacy_fixture is not None:
            (evidence/'legacy-before-input.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
            events_file = legacy_fixture.root/'probe-events.jsonl'
            with events_file.open('a') as log:
                log.write('{"event":"input_begin"}\n')
            adb('shell', 'input', 'tap', '1649', '562')  # MCPE use/place
            time.sleep(3)
            (evidence/'legacy-after-place.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
            adb('shell', 'input', 'tap', '1434', '691')  # MCPE attack/break
            time.sleep(3)
            (evidence/'legacy-after-break.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
            adb('shell', 'input', 'swipe', '233', '820', '233', '650', '1500')
            time.sleep(5)
            (evidence/'legacy-after-move.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
            records = [json.loads(line) for line in events_file.read_text().splitlines()]
            begin = next(i for i, record in enumerate(records) if record['event'] == 'input_begin')
            observed = {record['event'] for record in records[begin+1:]}
            legacy_verified = {'move', 'break', 'place'} <= observed
            (evidence/'legacy-interactions.json').write_text(json.dumps({
                'server':'Cuberite protocol 47', 'observed_after_touch':sorted(observed),
                'basic_interaction_verified':legacy_verified, 'pvp_verified':False}, indent=2))
            if not legacy_verified:
                raise RuntimeError('Legacy gameplay probe did not confirm move, break and place; inspect screenshots and server evidence')
            if os.environ.get('PROBE_PVP') == 'true':
                peer_log = (evidence/'legacy-peer.log').open('wb')
                legacy_peer = subprocess.Popen(
                    ['target/debug/examples/legacy_server_probe', '127.0.0.1:25565', '--peer'],
                    stdout=peer_log, stderr=subprocess.STDOUT)
                time.sleep(10)
                if legacy_peer.poll() is not None:
                    raise RuntimeError('Second protocol client exited; inspect legacy-peer.log')
                (evidence/'legacy-before-attack.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
                adb('shell', 'input', 'tap', '1434', '691')
                time.sleep(3)
                (evidence/'legacy-after-attack.png').write_bytes(adb('exec-out', 'screencap', '-p').stdout)
                records = [json.loads(line) for line in events_file.read_text().splitlines()]
                damage = [r for r in records if r.get('event') == 'pvp_damage'
                          and r.get('attacker') == 'LegacyAndroid' and r.get('damage', 0) > 0]
                pvp_attack_verified = bool(damage)
                (evidence/'legacy-pvp.json').write_text(json.dumps({
                    'attack_verified':pvp_attack_verified, 'damage_events':damage,
                    'opponent':'isolated protocol test client', 'full_combat_parity':False}, indent=2))
                if not pvp_attack_verified:
                    raise RuntimeError('Server did not confirm touch attack damage; inspect PvP evidence')
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
    if legacy_peer is not None:
        if legacy_peer.poll() is None:
            legacy_peer.terminate()
        legacy_peer.wait(timeout=10)
    if peer_log is not None:
        peer_log.close()
    if legacy_fixture is not None:
        legacy_fixture.close()
    (evidence/'logcat.txt').write_bytes(adb('logcat', '-d', check=False).stdout)
    (evidence/'startup.png').write_bytes(adb('exec-out', 'screencap', '-p', check=False).stdout)
    (evidence/'RESULT.json').write_text(json.dumps({'package':app,'apk':apk.name,'installed':installed,
        'startup_passed':startup_passed,'failure':failed,'local_profiles_verified':profiles_verified,
        'test':'x86_64 emulator API 35 legacy touch interaction' if legacy_fixture is not None else 'x86_64 emulator API 35 startup and optional UI exploration',
        'legacy_basic_interaction_verified':legacy_verified,'world_or_multiplayer_tested':legacy_verified,'fps_benchmark':False,
        'pvp_verified':False, 'pvp_attack_verified':pvp_attack_verified,
        'server_probe':os.environ.get('TEST_SERVER', 'none'),
        'public_server_probe':os.environ.get('TEST_SERVER') == 'mush',
        'screenshots_require_manual_gameplay_review':True},indent=2)+'\n')
if failed:
    raise SystemExit(failed)
