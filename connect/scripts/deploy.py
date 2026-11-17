"""Deploy with the existing Wrangler identity. Never print encryption material."""
import base64,json,os,secrets,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
wrangler=root.parent/'licensing/node_modules/wrangler/bin/wrangler.js'
local=root/'.local';local.mkdir(exist_ok=True)
backup=local/'connect-secrets.dpapi'
def dpapi(data,protect):
    method='Protect' if protect else 'Unprotect'
    decode='[Text.Encoding]::UTF8.GetBytes($inputText)' if protect else '[Convert]::FromBase64String($inputText)'
    output='[Convert]::ToBase64String($result)' if protect else '[Text.Encoding]::UTF8.GetString($result)'
    script=f'Add-Type -AssemblyName System.Security; $inputText=[Console]::In.ReadToEnd(); $result=[Security.Cryptography.ProtectedData]::{method}({decode},$null,[Security.Cryptography.DataProtectionScope]::CurrentUser); [Console]::Write({output})'
    result=subprocess.run(['powershell','-NoProfile','-NonInteractive','-Command',script],input=data,text=True,capture_output=True,check=True)
    return result.stdout
def run(args,cwd=root,input_data=None):
    result=subprocess.run(['node',str(wrangler),*args],cwd=cwd,input=input_data,text=True,capture_output=True)
    print(result.stdout,flush=True)
    if result.returncode:
        print(result.stderr)
        raise SystemExit(result.returncode)
probe=subprocess.run(['node',str(wrangler),'secret','list'],cwd=root,text=True,capture_output=True)
required={'AUTH_PEPPER','CRYPTO_KEY'}
if probe.returncode==0:
    names={item['name'] for item in json.loads(probe.stdout)}
    if required.issubset(names):
        values=None  # Ordinary deploys NEVER rotate or replace existing secrets.
    elif names.intersection(required):
        raise SystemExit('Incomplete remote key set. Restore deliberately; refusing automatic replacement.')
    elif backup.exists():
        values=json.loads(dpapi(backup.read_text(),False))
    elif '--initialize-empty' in sys.argv:
        values={name:base64.urlsafe_b64encode(secrets.token_bytes(32)).decode().rstrip('=') for name in required}
        backup.write_text(dpapi(json.dumps(values),True))
    else:
        raise SystemExit('No existing keys or local backup. Refusing implicit key generation.')
else:
    raise SystemExit('Cannot verify remote key state. Refusing a deployment that could replace keys.')
run(['d1','migrations','apply','dado-connect','--remote'])
run(['deploy'])
if values is not None:run(['secret','bulk'],input_data=json.dumps(values))
run(['deploy','--config','wrangler.admin.jsonc'],cwd=root.parent/'licensing')
print('Connect public service and protected admin deployed. Secrets backed up with Windows DPAPI.')
