/* 0041c6a8 FUN_0041c6a8 */

undefined4 __thiscall FUN_0041c6a8(void *this,HWND param_1,int param_2)

{
  undefined4 uVar1;
  
  if (param_2 == 0x110) {
    SetWindowTextA(param_1,"Logic Friday");
    uVar1 = 1;
  }
  else if (param_2 == 0x111) {
    *(undefined4 *)((int)this + 0xb4) = 1;
    EnableWindow(*(HWND *)this,1);
    DestroyWindow(param_1);
    *(undefined4 *)((int)this + 4) = 0;
    uVar1 = 1;
  }
  else {
    uVar1 = 0;
  }
  return uVar1;
}
