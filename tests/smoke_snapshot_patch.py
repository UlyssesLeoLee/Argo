#!/usr/bin/env python3
"""Real Git-only smoke for Argo cross-node archive / reviewed patch semantics.
No Rust binary or AI service is required. This is NOT a Rust integration test.
"""
from pathlib import Path
import subprocess,tempfile,tarfile,io

def git(root,*args):
    r=subprocess.run(['git','-C',str(root),*args],capture_output=True,check=True)
    return r.stdout

def main():
    with tempfile.TemporaryDirectory(prefix='argo-git-protocol-') as tmp:
        home=Path(tmp)
        src=home/'file_owner';src.mkdir()
        git(src,'init','-q');git(src,'config','user.email','argo@test');git(src,'config','user.name','Argo Test')
        (src/'src').mkdir();(src/'src'/'code.txt').write_text('initial\n')
        (src/'private.txt').write_text('NOT EXPORTED\n')
        git(src,'add','.');git(src,'commit','-q','-m','initial')
        sha=git(src,'rev-parse','HEAD').decode().strip()
        # Only approved scope exported; .git and non-scope files never cross nodes.
        archive=git(src,'archive','--format=tar','HEAD','--','src')
        runner=home/'executor';runner.mkdir()
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            for member in tar.getmembers():
                assert not member.issym() and not member.islnk()
                assert not Path(member.name).is_absolute() and '..' not in Path(member.name).parts
                tar.extract(member,runner,filter='data')
        assert (runner/'src'/'code.txt').read_text()=='initial\n'
        assert not (runner/'private.txt').exists()
        git(runner,'init','-q');git(runner,'config','user.email','argo@test');git(runner,'config','user.name','Argo Test')
        git(runner,'add','-A');git(runner,'commit','-q','-m','snapshot')
        (runner/'src'/'code.txt').write_text('updated by remote agent\n')
        patch=git(runner,'diff','HEAD','--binary')
        assert patch and b'src/code.txt' in patch
        assert git(src,'rev-parse','HEAD').decode().strip()==sha
        review=home/'approved_worktree';git(src,'worktree','add','-q','-b','argo/review-test',str(review),'HEAD')
        patch_file=home/'approved.patch';patch_file.write_bytes(patch)
        git(review,'apply','--check','--binary',str(patch_file))
        git(review,'apply','--binary',str(patch_file))
        assert (review/'src'/'code.txt').read_text()=='updated by remote agent\n'
        assert (src/'src'/'code.txt').read_text()=='initial\n'
        assert git(src,'rev-parse','HEAD').decode().strip()==sha
        print('PASS Git-only smoke: scope snapshot, source isolation, diff, owner review worktree')

if __name__=='__main__':main()
