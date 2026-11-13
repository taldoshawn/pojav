#!/usr/bin/env python3
"""Disposable Cuberite protocol-47 fixture, never a production server.

Binary release: 1.7EOL, commit 72aa50f710200832f97a0022dc70c5cd531b6aea,
Apache-2.0. Downloaded only into temporary test storage, never into an APK.
"""
import hashlib
import io
from pathlib import Path
import socket
import subprocess
import tarfile
import tempfile
import time
import urllib.request

URL = 'https://github.com/cuberite/cuberite/releases/download/1.7EOL/Cuberite-linux64.tar.gz'
SHA256 = '065753a7ed43fa5e6b567a84b7d0274c51289a75ab6bfa786208b256b0ff3759'

class LegacyServer:
    def __init__(self, evidence, port=25565):
        self.evidence = Path(evidence)
        self.evidence.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix='pojav-legacy-server-')
        self.root = Path(self.temp.name) / 'Server'
        self.process = None
        self.log = None
        self.port = port

    def start(self):
        with urllib.request.urlopen(URL, timeout=60) as response:
            archive = response.read(8 * 1024 * 1024)
        if hashlib.sha256(archive).hexdigest() != SHA256:
            raise RuntimeError('Cuberite fixture checksum mismatch')
        with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as tar:
            tar.extractall(self.temp.name, filter='data')
        (self.root/'settings.ini').write_text(
            f'[Server]\nPorts={self.port}\nMaxPlayers=2\n'
            '[Authentication]\nAuthenticate=0\n[Worlds]\nDefaultWorld=world\n'
            '[Plugins]\nPlugin=LegacyProbe\n')
        (self.root/'webadmin.ini').write_text('[WebAdmin]\nEnabled=0\n')
        world = self.root/'world'
        world.mkdir()
        (world/'world.ini').write_text(
            '[General]\nGamemode=1\nDimension=Overworld\n'
            '[Seed]\nSeed=1\n[SpawnPosition]\nX=0.5\nY=64\nZ=0.5\nPregenerateDistance=4\nMaxViewDistance=4\n'
            '[Generator]\nGenerator=Composable\nBiomeGen=Constant\nConstantBiome=Plains\n'
            'ShapeGen=HeightMap\nHeightGen=Flat\nFlatHeight=63\nCompositionGen=Classic\nFinishers=\n'
            '[Monsters]\nAnimalsOn=0\n')
        plugin = self.root/'Plugins/LegacyProbe'
        plugin.mkdir()
        (plugin/'main.lua').write_text('''
local main_x, main_y, main_z = 0.5, 64, 0.5
local test_world = nil
local function record(line)
  local f = io.open("probe-events.jsonl", "a")
  if f then f:write(line .. "\\n"); f:close() end
end
function Initialize(Plugin)
  Plugin:SetName("LegacyProbe")
  Plugin:SetVersion(1)
  cPluginManager:AddHook(cPluginManager.HOOK_PLAYER_SPAWNED, OnSpawn)
  cPluginManager:AddHook(cPluginManager.HOOK_PLAYER_MOVING, OnMove)
  cPluginManager:AddHook(cPluginManager.HOOK_PLAYER_BROKEN_BLOCK, OnBreak)
  cPluginManager:AddHook(cPluginManager.HOOK_PLAYER_PLACED_BLOCK, OnPlace)
  cPluginManager:AddHook(cPluginManager.HOOK_TAKE_DAMAGE, OnDamage)
  cPluginManager:AddHook(cPluginManager.HOOK_TICK, OnTick)
  return true
end
function OnSpawn(Player)
  record(string.format('{"event":"spawn","name":"%s"}', Player:GetName()))
  if Player:GetName() == "LegacyOpponent" then
    Player:SetGameMode(0)
    Player:TeleportToCoords(main_x, main_y, main_z + 2.0)
    return false
  end
  Player:SetYaw(0)
  Player:SetPitch(0)
  test_world = Player:GetWorld()
  Player:GetInventory():SetHotbarSlot(0, cItem(1, 64))
  Player:GetWorld():SetBlock(0, 65, 3, 1, 0)
  return false
end
function OnMove(Player, Old, New)
  if Player:GetName() ~= "LegacyOpponent" then
    main_x, main_y, main_z = New.x, New.y, New.z
  end
  if (Old - New):SqrLength() > 0.0001 then
    record(string.format('{"event":"move","x":%.4f,"y":%.4f,"z":%.4f}', New.x, New.y, New.z))
  end
  return false
end
function OnBreak(Player, X, Y, Z)
  record(string.format('{"event":"break","x":%d,"y":%d,"z":%d}', X, Y, Z))
  return false
end
function OnPlace(Player, X, Y, Z)
  record(string.format('{"event":"place","x":%d,"y":%d,"z":%d}', X, Y, Z))
  return false
end
function OnDamage(Receiver, TDI)
  if Receiver:IsPlayer() and Receiver:GetName() == "LegacyOpponent"
      and TDI.Attacker and TDI.Attacker:IsPlayer() then
    record(string.format('{"event":"pvp_damage","attacker":"%s","receiver":"%s","damage":%.4f}',
      TDI.Attacker:GetName(), Receiver:GetName(), TDI.FinalDamage))
  end
  return false
end
function OnTick()
  if not test_world then return false end
  local f = io.open("probe-command.txt", "r")
  if not f then return false end
  local command = f:read("*a"); f:close()
  os.remove("probe-command.txt")
  if command == "reset_blocks" then
    for x = -1, 1 do
      for z = 0, 5 do test_world:SetBlock(x, 65, z, 0, 0) end
    end
    test_world:SetBlock(0, 65, 3, 1, 0)
    test_world:ForEachPlayer(function(Player)
      if Player:GetName() ~= "LegacyOpponent" then
        Player:SetYaw(0); Player:SetPitch(0)
        Player:TeleportToCoords(0.5, 64, 0.5)
      end
      return false
    end)
    record('{"event":"fixture_reset"}')
  elseif command == "show_pickup" then
    local items = cItems()
    items:Add(cItem(264, 1))
    test_world:SpawnItemPickups(items, main_x + 1.0, main_y + 0.5, main_z + 4.0, 0)
    record('{"event":"pickup_spawned","item":"diamond"}')
  end
  return false
end
''')
        self.log = (self.evidence/'legacy-server.log').open('wb')
        self.process = subprocess.Popen(['./Cuberite'], cwd=self.root,
            stdin=subprocess.PIPE, stdout=self.log, stderr=subprocess.STDOUT)
        for _ in range(60):
            if self.process.poll() is not None:
                raise RuntimeError('Legacy fixture exited during startup')
            try:
                with socket.create_connection(('127.0.0.1', self.port), timeout=1):
                    return
            except OSError:
                pass
            time.sleep(1)
        raise RuntimeError('Legacy server startup timed out')

    def close(self):
        if self.process is not None and self.process.poll() is None:
            try:
                self.process.communicate(b'stop\n', timeout=15)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        if self.log is not None:
            self.log.close()
        events = self.root/'probe-events.jsonl'
        if events.exists():
            (self.evidence/'legacy-events.jsonl').write_bytes(events.read_bytes())
        self.temp.cleanup()

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--probe', required=True, type=Path)
    parser.add_argument('--evidence', type=Path, default=Path('.local/legacy-server-evidence'))
    args = parser.parse_args()
    server = LegacyServer(args.evidence, 25566)
    try:
        server.start()
        subprocess.run([str(args.probe.resolve()), '127.0.0.1:25566'], check=True, timeout=45)
    finally:
        server.close()
