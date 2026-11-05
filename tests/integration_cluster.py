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
            # The source node owns a real Git checkout; executor B owns no checkout.
            source_repo=pathlib.Path(directory)/'file-owner';source_repo.mkdir()
            subprocess.run(['git','-C',str(source_repo),'init','-q'],check=True)
            subprocess.run(['git','-C',str(source_repo),'config','user.name','Argo E2E'],check=True)
            subprocess.run(['git','-C',str(source_repo),'config','user.email','argo@localhost'],check=True)
            (source_repo/'src').mkdir();(source_repo/'src'/'hello.txt').write_text('original\n')
            subprocess.run(['git','-C',str(source_repo),'add','-A'],check=True)
            subprocess.run(['git','-C',str(source_repo),'commit','-qm','initial'],check=True)
            for nid in ('machine-a','machine-b'):
                values={**env,'ARGO_NODE_ID':nid,'ARGO_REPOS':f'project={source_repo}' if nid=='machine-a' else ''}
                procs.append(subprocess.Popen(command+['worker'],cwd=ROOT,env=values,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL))
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
            # Two-node snapshot: owner A exports only src/; B runs with the snapshot.
            remote=call(host,'/api/v1/tasks',token_admin,'POST',{'title':'Remote source snapshot','prompt':'Edit the scoped file',
                'provider':'claude','target_node':'machine-b','repo_alias':'project',
                'context':{'argo_file_owner':'machine-a','argo_file_scope':'src'}})
            done_remote=eventually(lambda:call(host,'/api/v1/tasks/'+remote['id'],token_admin),
                lambda t:t['state']=='completed' and t['source_state']=='ready',timeout=50)
            assert done_remote['assigned_node']=='machine-b'
            assert done_remote['source_sha'] and done_remote['source_commit']
            assert (source_repo/'src'/'hello.txt').read_text()=='original\n'
            # Multica metadata adapter: simulated stdout but still validates the transport.
            discovery=call(host,'/api/v1/tasks',token_admin,'POST',{'provider':'multica','title':'Catalog','prompt':'Read projects',
               'context':{'multica_workspace_id':'w','argo_action':'discover_projects'}})
            eventually(lambda:call(host,'/api/v1/tasks/'+discovery['id'],token_admin),lambda t:t['state']=='completed')
            try:call(host,'/api/v1/tasks',token_admin+'WRONG','POST',payload)
            except urllib.error.HTTPError as e:assert e.code==401
            else:raise AssertionError('Missing authorization enforcement')
            print('PASS E2E: 2 workers, pinned node, owner snapshot + remote execution, blob SHA, events, Multica catalog, auth')
        finally:
            for p in reversed(procs):
                p.terminate()
                try:p.wait(timeout=3)
                except subprocess.TimeoutExpired:p.kill()
if __name__=='__main__':main()
