/* 0041a350 FUN_0041a350 */

void __fastcall FUN_0041a350(int param_1)

{
  undefined1 local_2c [12];
  undefined4 local_20;
  undefined4 local_1c;
  
  local_1c = 2;
  local_20 = 2;
  SendMessageA(*(HWND *)(param_1 + 0x14),0x102b,0xffffffff,(LPARAM)local_2c);
  SetFocus(*(HWND *)(param_1 + 0x14));
  return;
}
