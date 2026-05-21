/* 00439af4 FUN_00439af4 */

void __fastcall FUN_00439af4(undefined4 *param_1)

{
  HANDLE hMem;
  SIZE_T _Size;
  uint *_Memory;
  uint *puVar1;
  
  OpenClipboard((HWND)*param_1);
  hMem = GetClipboardData(1);
  if (hMem == (HANDLE)0x0) {
    CloseClipboard();
  }
  else {
    _Size = GlobalSize(hMem);
    _Memory = _malloc(_Size);
    puVar1 = GlobalLock(hMem);
    FUN_0043ebd0(_Memory,puVar1);
    GlobalUnlock(hMem);
    CloseClipboard();
    SendMessageA((HWND)param_1[1],0xc2,1,(LPARAM)_Memory);
    _free(_Memory);
    FUN_00439a35((int)param_1);
  }
  return;
}
