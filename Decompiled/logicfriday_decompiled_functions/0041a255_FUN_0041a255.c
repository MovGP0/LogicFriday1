/* 0041a255 FUN_0041a255 */

void __thiscall FUN_0041a255(void *this,int *param_1)

{
  undefined4 *local_20;
  undefined4 local_18;
  int local_14;
  int local_c;
  int local_8;
  
  SendMessageA(*(HWND *)((int)this + 0x10),0x1013,0,0);
  if (*param_1 != 0) {
    if (&stack0x00000000 == (undefined1 *)0x18) {
      local_20 = (undefined4 *)0x0;
    }
    else {
      local_18 = 0;
      local_20 = &local_18;
    }
    SendMessageA(*(HWND *)((int)this + 0x10),0x100e,0,(LPARAM)local_20);
    local_8 = *param_1 * (local_c - local_14);
    SendMessageA(*(HWND *)((int)this + 0x10),0x1014,0,local_8);
  }
  return;
}
