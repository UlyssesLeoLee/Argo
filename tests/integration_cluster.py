#!/usr/bin/env python3
"""Argo E2E (no vendor credentials): controller + two mocked Rust workers.
Requirements: a compiled argo-runtime binary, or cargo on PATH. No external Python packages.
Usage: python tests/integration_cluster.py [path/to/argo-runtime[.exe]]
"""
import hashlib, json, os, pathlib, shutil, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request
ROOT=pathlib.Path(__file__).resolve().parents[1]
def free_port():
    with socket.socket() as sock: sock.bind(('127.0.0.1',0));return sock.getsockname()[1]
def call(host,route,token,method='GET',payload=None,blob=False):
    data=payload if blob else (json.dumps(payload).encode() if payload is not None else None)
    req=urllib.request.Request(host+route,data=data,headers={'Authorization':'Bearer '+token,**({'Content-Type':'application/json'} if data is not None and not blob else {})},method=method)
    with urllib.request.urlopen(req,timeout=7) as resp:return json.loads(resp.read())
def eventually(action,condition,timeout=25):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        try:
            v=action()
            if condition(v):return v
        except (urllib.error.URLError,TimeoutError):pass
        time.sleep(.25)
    raise AssertionError('condition not met before timeout')
def main():
    supplied=sys.argv[1] if len(sys.argv)>1 else None
    if supplied: command=[supplied]
    elif shutil.which('cargo'):command=['cargo','run','--quiet','-p','argo-runtime','--']
    else:raise SystemExit('SKIP: Rust toolchain not present; compile with cargo first')
    token_admin='test-admin-0123456789abcdef0123456789';token_worker='test-worker-fedcba9876543210fedcba9876543210'
    with tempfile.TemporaryDirectory(prefix='argo-e2e-') as directory:
        port=free_port();host=f'http://127.0.0.1:{port}'
        env={**os.environ,'ARGO_ADMIN_TOKEN':token_admin,'ARGO_WORKER_TOKEN':token_worker,'ARGO_BIND':f'127.0.0.1:{port}',
             'ARGO_DATA_DIR':directory,'ARGO_WEB_DIR':str(ROOT/'web'),'ARGO_SERVER':host,'ARGO_DEMO_MODE':'1', 'ARGO_RUNS_DIR':directory}
        procs=[]
        try:
            server=subprocess.Popen(command+['server'],cwd=ROOT,env=env,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
            procs.append(server)
            eventually(lambda:urllib.request.urlopen(host+'/health',timeout=2).status,lambda s:s==200,timeout=40)
            for nid in ('machine-a','machine-b'):
                procs.append(subprocess.Popen(command+['worker'],cwd=ROOT,env={**env,'ARGO_NODE_ID':nid},stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL))
            nodes=eventually(lambda:call(host,'/api/v1/nodes',token_admin),lambda n:len(n)>=2)
            assert {n['id'] for n in nodes}=={'machine-a','machine-b'}
            b=b'Argo tests binary blob\n';uploaded=call(host,'/api/v1/blobs',token_admin,'POST',b,blob=True)
            assert uploaded['sha256']==hashlib.sha256(b).hexdigest()
            payload={'title':'Cross node prompt','prompt':'Hello world','provider':'codex','target_node':'machine-b','context':{},
                     'attachments':[{'name':'input.txt','sha256':uploaded['sha256']}]}
            task=call(host,'/api/v1/tasks',token_admin,'POST',payload)
            done=eventually(lambda:call(host,'/api/v1/tasks/'+task['id'],token_admin),lambda t:t['state']=='completed')
            assert done['assigned_node']=='machine-b'
            events=call(host,f"/api/v1/tasks/{task['id']}/events?since=0",token_admin)
            assert len(events)>=1
            # Multica metadata adapter: simulated stdout but still validates the transport.
            discovery=call(host,'/api/v1/tasks',token_admin,'POST',{'provider':'multica','title':'Catalog','prompt':'Read projects',
               'context':{'multica_workspace_id':'w','argo_action':'discover_projects'}})
            eventually(lambda:call(host,'/api/v1/tasks/'+discovery['id'],token_admin),lambda t:t['state']=='completed')
            try:call(host,'/api/v1/tasks',token_admin+'WRONG','POST',payload)
            except urllib.error.HTTPError as e:assert e.code==401
            else:raise AssertionError('Missing authorization enforcement')
            print('PASS E2E: 2 workers, pinned node, blob SHA, events, Multica catalog, auth')
        finally:
            for p in reversed(procs):
                p.terminate()
                try:p.wait(timeout=3)
                except subprocess.TimeoutExpired:p.kill()
if __name__=='__main__':main()
