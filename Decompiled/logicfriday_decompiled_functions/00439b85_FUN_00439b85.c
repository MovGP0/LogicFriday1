/* 00439b85 FUN_00439b85 */

void __thiscall FUN_00439b85(void *this,undefined4 *param_1)

{
  LRESULT LVar1;
  BOOL BVar2;
  undefined4 local_14;
  undefined4 local_10;
  int local_c;
  int local_8;
  
  local_14 = 0;
  local_10 = 0;
  LVar1 = SendMessageA(*(HWND *)((int)this + 4),0xc6,0,0);
  if (LVar1 != 0) {
    *param_1 = 1;
  }
  LVar1 = SendMessageA(*(HWND *)((int)this + 4),0x455,0,0);
  if (LVar1 != 0) {
    param_1[1] = 1;
  }
  LVar1 = SendMessageA(*(HWND *)((int)this + 4),0x45f,(WPARAM)&local_14,0);
  if (LVar1 != 0) {
    param_1[2] = 1;
    SendMessageA(*(HWND *)((int)this + 4),0x434,0,(LPARAM)&local_c);
    if (local_8 != local_c) {
      param_1[3] = 1;
    }
  }
  BVar2 = IsClipboardFormatAvailable(1);
  param_1[4] = BVar2;
  return;
}
