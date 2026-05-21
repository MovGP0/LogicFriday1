/* 00417cde FUN_00417cde */

void __fastcall FUN_00417cde(int param_1)

{
  undefined4 local_8;
  
  SendMessageA(*(HWND *)(param_1 + 0x14),0x1009,0,0);
  SendMessageA(*(HWND *)(param_1 + 0x14),0x102f,0,0);
  for (local_8 = *(WPARAM *)(param_1 + 0x68); 0 < (int)local_8; local_8 = local_8 - 1) {
    SendMessageA(*(HWND *)(param_1 + 0x14),0x101c,local_8,0);
  }
  *(undefined4 *)(param_1 + 0x68) = 1;
  _memset((void *)(param_1 + 0x9c),0,0x40);
  return;
}
