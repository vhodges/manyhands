#ifndef RetryCount
  #error RetryCount must come from the production definition
#endif
#ifndef RetryDelay
  #error RetryDelay must come from the production definition
#endif

[Setup]
AppId=io.github.vhodges.manyhands.retry-test
AppName=Manyhands mandatory signing retry test
AppVersion=0.0.0
DefaultDirName={tmp}\unused-retry-fixture
PrivilegesRequired=lowest
SetupArchitecture=x64
Uninstallable=no
CreateAppDir=no
OutputBaseFilename=installer-retry-test
SignTool=fixture
SignToolRetryCount={#RetryCount}
SignToolRetryDelay={#RetryDelay}
