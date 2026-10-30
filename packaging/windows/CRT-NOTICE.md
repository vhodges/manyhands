# Microsoft C/C++ runtime

Windows releases link the Microsoft MSVC C runtime statically. Microsoft
runtime components retain their applicable Microsoft distributable-code
terms; Manyhands' MIT licence does not relicense those components.

Microsoft Visual Studio 2022 distributable-code documentation:
<https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution>

Microsoft Visual C++ runtime licence terms:
<https://visualstudio.microsoft.com/license-terms/vs2022-cruntime/>

The release does not require a separate Visual C++ runtime installer.
Its import audit rejects dynamic CRT dependencies and unreviewed DLLs.
