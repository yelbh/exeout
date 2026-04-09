$proc = Start-Process npx -ArgumentList "tauri signer generate" -PassThru -RedirectStandardOutput "new_keys.txt" -RedirectStandardError "new_keys_err.txt" -RedirectStandardInput "stdin.txt"
Start-Sleep -Seconds 2
Add-Content -Path "stdin.txt" -Value "`n"
Start-Sleep -Seconds 1
Add-Content -Path "stdin.txt" -Value "`n"
Start-Sleep -Seconds 5
