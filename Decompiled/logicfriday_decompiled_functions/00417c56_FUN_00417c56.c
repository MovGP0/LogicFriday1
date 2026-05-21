/* 00417c56 FUN_00417c56 */

void __fastcall FUN_00417c56(int param_1)

{
  undefined4 local_8;
  
  SendMessageA(*(HWND *)(param_1 + 0x10),0x1009,0,0);
  for (local_8 = *(WPARAM *)(param_1 + 0x6c); 0 < (int)local_8; local_8 = local_8 - 1) {
    SendMessageA(*(HWND *)(param_1 + 0x10),0x101c,local_8,0);
  }
  *(undefined4 *)(param_1 + 0x6c) = 1;
  if (*(int *)(param_1 + 0x7c) != 0) {
    _free(*(void **)(param_1 + 0x7c));
    *(undefined4 *)(param_1 + 0x7c) = 0;
    _memset((void *)(param_1 + 0x70),0,0x1c);
  }
  return;
}
