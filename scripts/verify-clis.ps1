$ErrorActionPreference = 'Continue'
$progs = @('git','claude','codex','hermes','orca','multica','cargo','rustc','node')
foreach ($p in $progs) { $cmd = Get-Command $p -ErrorAction SilentlyContinue; if ($cmd) { Write-Host "[OK] $p : $($cmd.Source)" } else { Write-Host "[MISSING] $p" } }
Write-Host 'MiniMax uses Hermes provider=minimax. STAR uses an explicitly configured HTTPS webhook.'
