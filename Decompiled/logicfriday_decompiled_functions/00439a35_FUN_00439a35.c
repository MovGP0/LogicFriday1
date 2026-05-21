/* 00439a35 FUN_00439a35 */

void __fastcall FUN_00439a35(int param_1)

{
  *(undefined4 *)(param_1 + 0x28) = 0xdf0000;
  *(undefined4 *)(param_1 + 0x18) = 0x40000000;
  SendMessageA(*(HWND *)(param_1 + 4),0x444,4,param_1 + 0x14);
  SendMessageA(*(HWND *)(param_1 + 4),0xcf,0,0);
  SendMessageA(*(HWND *)(param_1 + 4),0xb1,0xffffffff,-1);
  SendMessageA(*(HWND *)(param_1 + 4),0xb1,0xffffffff,-1);
  return;
}
