/* 0040f300 FUN_0040f300 */

void __thiscall FUN_0040f300(void *this,WPARAM param_1)

{
  undefined1 local_58 [12];
  undefined4 local_4c;
  undefined4 local_48;
  undefined1 local_30 [12];
  undefined4 local_24;
  undefined4 local_20;
  WPARAM local_8;
  
  for (local_8 = 0; (int)local_8 < *(int *)((int)this + 0x50); local_8 = local_8 + 1) {
    local_20 = 2;
    local_24 = 0;
    SendMessageA(*(HWND *)((int)this + 4),0x102b,local_8,(LPARAM)local_30);
  }
  local_48 = 2;
  local_4c = 2;
  SendMessageA(*(HWND *)((int)this + 4),0x102b,param_1,(LPARAM)local_58);
  *(WPARAM *)((int)this + 0x54) = param_1;
  SendMessageA(*(HWND *)((int)this + 4),0x1013,param_1,0);
  return;
}
