/* 004398bb FUN_004398bb */

void __fastcall FUN_004398bb(int param_1)

{
  LRESULT lParam;
  
  lParam = SendMessageA(*(HWND *)(param_1 + 4),0xba,0,0);
  SendMessageA(*(HWND *)(param_1 + 4),0xb6,0,lParam);
  SendMessageA(*(HWND *)(param_1 + 4),0xb5,2,0);
  return;
}
