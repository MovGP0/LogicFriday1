/* 00439aab FUN_00439aab */

void __fastcall FUN_00439aab(int param_1)

{
  *(undefined4 *)(param_1 + 0x1c) = 0;
  *(undefined4 *)(param_1 + 0x18) = 0x10;
  SendMessageA(*(HWND *)(param_1 + 4),0x444,4,param_1 + 0x14);
  SendMessageA(*(HWND *)(param_1 + 4),0xc,0,0x44ad26);
  return;
}
