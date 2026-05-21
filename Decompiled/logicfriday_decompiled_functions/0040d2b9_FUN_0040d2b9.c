/* 0040d2b9 FUN_0040d2b9 */

void __cdecl FUN_0040d2b9(WPARAM param_1,int param_2)

{
  if (param_2 == 0) {
    SendMessageA(DAT_00452a24,0x411,param_1,0x10);
  }
  else if (param_2 == 1) {
    SendMessageA(DAT_00452a24,0x411,param_1,4);
  }
  return;
}
