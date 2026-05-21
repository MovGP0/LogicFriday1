/* 0041c712 FUN_0041c712 */

bool __fastcall FUN_0041c712(int param_1)

{
  BOOL BVar1;
  tagMSG local_20;
  
  while ((*(int *)(param_1 + 0xb4) == 0 &&
         (BVar1 = PeekMessageA(&local_20,(HWND)0x0,0,0,1), BVar1 != 0))) {
    if ((*(int *)(param_1 + 4) == 0) ||
       (BVar1 = IsDialogMessageA(*(HWND *)(param_1 + 4),&local_20), BVar1 == 0)) {
      TranslateMessage(&local_20);
      DispatchMessageA(&local_20);
    }
  }
  return *(int *)(param_1 + 0xb4) == 0;
}
