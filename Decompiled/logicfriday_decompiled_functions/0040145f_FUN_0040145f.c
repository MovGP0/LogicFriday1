/* 0040145f FUN_0040145f */

WPARAM FUN_0040145f(HINSTANCE param_1,undefined4 param_2,char *param_3)

{
  DWORD DVar1;
  HMODULE pHVar2;
  size_t sVar3;
  BOOL BVar4;
  int iVar5;
  INITCOMMONCONTROLSEX local_5c;
  tagMSG local_54;
  HACCEL local_38;
  WNDCLASSEXA local_34;
  
  DAT_00452914 = param_1;
  LoadStringA(param_1,0x67,&DAT_004528b0,100);
  LoadStringA(param_1,0x6d,&DAT_00452a30,100);
  local_5c.dwICC = 1;
  local_5c.dwSize = 8;
  InitCommonControlsEx(&local_5c);
  local_34.cbSize = 0x30;
  local_34.style = 0;
  local_34.lpfnWndProc = FUN_004016d2;
  local_34.cbClsExtra = 0;
  local_34.cbWndExtra = 0;
  local_34.hInstance = param_1;
  local_34.hIcon = LoadIconA(param_1,(LPCSTR)0xd6);
  local_34.hCursor = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f00);
  DVar1 = GetSysColor(0x1e);
  if (DVar1 == 0) {
    local_34.hbrBackground = (HBRUSH)0x5;
  }
  else {
    local_34.hbrBackground = (HBRUSH)0x1f;
  }
  local_34.lpszMenuName = (LPCSTR)0x6d;
  local_34.lpszClassName = &DAT_00452a30;
  local_34.hIconSm = (HICON)0x0;
  RegisterClassExA(&local_34);
  local_34.lpfnWndProc = FUN_0040b004;
  local_34.hbrBackground = GetStockObject(0);
  local_34.lpszClassName = "DiagOutWindow";
  RegisterClassExA(&local_34);
  local_34.lpfnWndProc = FUN_0040afc3;
  local_34.style = 0x28;
  local_34.lpszClassName = "DiagInWindow";
  RegisterClassExA(&local_34);
  pHVar2 = LoadLibraryA("RICHED20.DLL");
  if (pHVar2 == (HMODULE)0x0) {
    LoadLibraryA("RICHED32.DLL");
  }
  DAT_00452e70 = (uint)(pHVar2 != (HMODULE)0x0);
  DAT_00452aac = CreateWindowExA(0,&DAT_00452a30,&DAT_004528b0,0x4cf0000,-0x80000000,0,800,600,
                                 (HWND)0x0,(HMENU)0x0,param_1,(LPVOID)0x0);
  if (DAT_00452aac == (HWND)0x0) {
    local_54.wParam = 0;
  }
  else {
    local_38 = LoadAcceleratorsA(param_1,(LPCSTR)0x6d);
    sVar3 = _strlen(param_3);
    if (sVar3 != 0) {
      sVar3 = _strlen(param_3);
      lstrcpynA(&DAT_00452920,param_3 + 1,sVar3 - 1);
      PostMessageA(DAT_00452aac,0x111,0x8023,0);
    }
    while (BVar4 = GetMessageA(&local_54,(HWND)0x0,0,0), BVar4 != 0) {
      if ((((DAT_00452ac8 == (HWND)0x0) ||
           (BVar4 = IsDialogMessageA(DAT_00452ac8,&local_54), BVar4 == 0)) &&
          ((DAT_00452ac4 == (HWND)0x0 ||
           (BVar4 = IsDialogMessageA(DAT_00452ac4,&local_54), BVar4 == 0)))) &&
         (iVar5 = TranslateAcceleratorA(local_54.hwnd,local_38,&local_54), iVar5 == 0)) {
        TranslateMessage(&local_54);
        DispatchMessageA(&local_54);
        if ((local_54.message == 0x100) && (local_54.wParam == 0x1b)) {
          PostMessageA(DAT_00452aac,0x111,0x150,0);
        }
      }
    }
  }
  return local_54.wParam;
}
