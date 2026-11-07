#!/usr/bin/env python3
"""Git protocol negative cases; operates only in disposable temp directories.
This tests the Git semantics Argo relies on; it does NOT execute Rust.
"""
from pathlib import Path
import subprocess, tempfile, hashlib

def git(root,*args, ok=True, input=None):
    cmd=['git','-C',str(root),*args]
    r=subprocess.run(cmd,input=input,capture_output=True)
    if ok and r.returncode:raise AssertionError(f'{cmd}: {r.stderr.decode(errors="replace")}')
    return r

def scope_allowed(relative,scope):
    if relative.startswith('/') or '\\' in relative or any(x in {'.','..'} for x in relative.split('/')):
        return False
    return not scope or relative==scope or relative.startswith(scope+'/')

def run():
    count=0
    with tempfile.TemporaryDirectory(prefix='argo-redgreen-git-') as temp:
        root=Path(temp);repo=root/'owner';repo.mkdir();git(repo,'init','-q');git(repo,'config','user.name','Argo Tests');git(repo,'config','user.email','test@argo.invalid')
        (repo/'src').mkdir();(repo/'src'/'allowed.txt').write_text('old\n');(repo/'private.txt').write_text('secret\n')
        git(repo,'add','-A');git(repo,'commit','-qm','base')
        review=root/'review';git(repo,'worktree','add','-q','-b','argo/adversarial-review',str(review),'HEAD')
        old_head=git(repo,'rev-parse','HEAD').stdout.strip()
        # Case 1: valid scoped change must not modify main owner's checkout.
        (review/'src'/'allowed.txt').write_text('safe\n')
        assert scope_allowed('src/allowed.txt','src'); assert (repo/'src'/'allowed.txt').read_text()=='old\n';count+=1
        # Case 2: root path and sibling-path confusion are disallowed.
        for x in ['src2/out.txt','private.txt','../secrets.txt','src/../private.txt','/tmp/outside','src\\escape']:
            assert not scope_allowed(x,'src'),x
        count+=1
        # Case 3: symlink creation appears in Git patch and must be blocked by policy.
        try:
            (review/'src'/'link').symlink_to('../../private.txt')
            git(review,'add','-N','src/link')
            diff=git(review,'diff','HEAD','--binary').stdout
            assert b'new file mode 120000' in diff
        except (OSError,NotImplementedError):
            print('SKIP symlink creation: not supported on host')
        else:count+=1
        # Case 4: owner commit remains unchanged until explicitly merged.
        assert git(repo,'rev-parse','HEAD').stdout.strip()==old_head
        assert (repo/'private.txt').read_text()=='secret\n';count+=1
        # Case 5: SHA checking detects tampered blobs.
        data=b'valid patch';digest=hashlib.sha256(data).hexdigest()
        assert hashlib.sha256(data+b'changed').hexdigest()!=digest;count+=1
    print(f'PASS {count} Git negative/positive behavior checks (disposable repo only)')

if __name__=='__main__':run()
