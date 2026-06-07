param(
    [switch]$Release,
    [switch]$Clean
)

$env:Path = "C:\Users\mhedg\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin;" + [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

if ($Clean) {
    Write-Host "Cleaning..."
    cargo clean
}

$buildArgs = @("build")
if ($Release) {
    $buildArgs += "--release"
}
$buildArgs += $args

Write-Host "Running: cargo $($buildArgs -join ' ')"
cargo $buildArgs
