Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap(32,32)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::FromArgb(26,26,46))
$brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(14,165,233))
$g.FillEllipse($brush, 4, 4, 24, 24)
$g.Dispose()
$icon = [System.Drawing.Icon]::FromHandle($bmp.GetHicon())
$target = Join-Path $PSScriptRoot "src-tauri\icons\icon.ico"
$fs = [System.IO.FileStream]::new($target, [System.IO.FileMode]::Create)
$icon.Save($fs)
$fs.Close()
$bmp.Dispose()
Write-Output "Icon created successfully"
