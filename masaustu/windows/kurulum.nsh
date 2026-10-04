; Orhunca Stüdyo kurulumu: `orhunca` komutu (kurulum klasöründeki orhunca.exe) kullanıcının
; PATH'ine eklenir; kaldırırken çıkarılır. PowerShell, ortam değişikliğini açık
; pencerelere de duyurur.

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog `powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d='$INSTDIR'; $$p=[Environment]::GetEnvironmentVariable('Path','User'); if(-not $$p){$$p=''}; if(-not ($$p.Split(';') -contains $$d)){[Environment]::SetEnvironmentVariable('Path', (($$p.TrimEnd(';')+';'+$$d).TrimStart(';')), 'User')}"`
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::ExecToLog `powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d='$INSTDIR'; $$p=[Environment]::GetEnvironmentVariable('Path','User'); if($$p){[Environment]::SetEnvironmentVariable('Path', (($$p.Split(';') | Where-Object { $$_ -and $$_ -ne $$d }) -join ';'), 'User')}"`
!macroend
