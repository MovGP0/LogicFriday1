/* 0043a79e FUN_0043a79e */

undefined4 __thiscall
FUN_0043a79e(void *this,HWND param_1,undefined4 param_2,uint param_3,undefined4 param_4)

{
  *(undefined4 *)((int)this + 0x44) = 1;
  SetCapture(param_1);
  FUN_0043a5a9(this,(int)(short)param_4,(int)(short)((uint)param_4 >> 0x10),param_3);
  return 0;
}
