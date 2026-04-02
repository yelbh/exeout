Add-Type -AssemblyName System.Drawing

$iconDir = 'C:\aserveur\www\exephp\src-tauri\icons'
New-Item -ItemType Directory -Force -Path $iconDir | Out-Null

$src = 'C:\Users\DELL\.gemini\antigravity\brain\b008ecf4-bafc-4257-8374-17465a2fb223\exeoutput_icon_1774347153669.png'
$img = [System.Drawing.Image]::FromFile($src)

function Save-Resized($size, $outName) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.DrawImage($img, 0, 0, $size, $size)
    $g.Dispose()
    $bmp.Save((Join-Path $iconDir $outName), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

Save-Resized 32 '32x32.png'
Save-Resized 128 '128x128.png'
Save-Resized 256 '128x128@2x.png'
Save-Resized 128 'icon.icns'

$img.Dispose()

# Build a valid ICO file from the 32x32 PNG
$pngBytes = [System.IO.File]::ReadAllBytes((Join-Path $iconDir '32x32.png'))
$icoPath = Join-Path $iconDir 'icon.ico'
$ms = New-Object System.IO.MemoryStream

# ICONDIR header: reserved=0, type=1, count=1
[void]$ms.Write([byte[]](0,0,1,0,1,0), 0, 6)

# ICONDIRENTRY: width, height, colors, reserved, planes, bitcount, size (4 bytes), offset (4 bytes)
$dataSize = [System.BitConverter]::GetBytes([int]$pngBytes.Length)
$offset   = [System.BitConverter]::GetBytes([int]22)  # 6 + 16 = 22

$ms.WriteByte(32); $ms.WriteByte(32); $ms.WriteByte(0); $ms.WriteByte(0)
[void]$ms.Write([byte[]](1,0,32,0), 0, 4)
[void]$ms.Write($dataSize, 0, 4)
[void]$ms.Write($offset, 0, 4)

# PNG data
[void]$ms.Write($pngBytes, 0, $pngBytes.Length)

[System.IO.File]::WriteAllBytes($icoPath, $ms.ToArray())
$ms.Dispose()

Write-Host "Icons created successfully:"
Get-ChildItem $iconDir | Select-Object Name, Length | Format-Table -AutoSize
