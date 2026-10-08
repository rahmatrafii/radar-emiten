param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$Message,

    [Parameter(Position=1)]
    [string]$From = "6281234567890"
)

# 1. Baca secret dari .env
$envFile = Join-Path $PSScriptRoot "..\\.env"
$secret = $env:WHATSAPP_APP_SECRET
if ([string]::IsNullOrEmpty($secret) -and (Test-Path $envFile)) {
    Get-Content $envFile | ForEach-Object {
        if ($_ -match "^\s*WHATSAPP_APP_SECRET\s*=\s*(.+)$") {
            $secret = $matches[1].Trim()
        }
    }
}
if ([string]::IsNullOrEmpty($secret)) {
    Write-Host "Error: WHATSAPP_APP_SECRET tidak ditemukan di .env atau environment variable." -ForegroundColor Red
    exit 1
}

# 2. Susun payload Meta Webhook format resmi
$wamid = "wamid.HBgL" + [System.Guid]::NewGuid().ToString().Replace("-", "")
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds().ToString()

$payloadJson = @"
{"object":"whatsapp_business_account","entry":[{"id":"1","changes":[{"field":"messages","value":{"messaging_product":"whatsapp","metadata":{"phone_number_id":"123"},"messages":[{"id":"$wamid","from":"$From","timestamp":"$timestamp","type":"text","text":{"body":"$Message"}}]}}]}]}
"@

# 3. Hitung HMAC-SHA256 signature
$hmac = New-Object System.Security.Cryptography.HMACSHA256
$hmac.Key = [System.Text.Encoding]::UTF8.GetBytes($secret)
$hashBytes = $hmac.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($payloadJson))
$hashHex = [System.BitConverter]::ToString($hashBytes).Replace("-", "").ToLower()
$signatureHeader = "sha256=" + $hashHex

Write-Host "Mengirim pesan WhatsApp dari $($From): `"$Message`"" -ForegroundColor Cyan

# 4. Kirim ke Webhook
try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/webhook/whatsapp" `
        -Method Post `
        -Headers @{ "x-hub-signature-256" = $signatureHeader } `
        -Body $payloadJson `
        -ContentType "application/json"

    Write-Host "Webhook Response: " -NoNewline
    Write-Host ($response | ConvertTo-Json -Compress) -ForegroundColor Green
} catch {
    Write-Host "Gagal mengirim webhook: $_" -ForegroundColor Red
}
