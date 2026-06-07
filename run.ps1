param(
    [switch]$Release
)

$env:Path = "C:\Users\mhedg\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin;" + [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

$runArgs = @("run")
if ($Release) {
    $runArgs += "--release"
}
$runArgs += $args

Write-Host "Running: cargo $($runArgs -join ' ')"
cargo $runArgs
