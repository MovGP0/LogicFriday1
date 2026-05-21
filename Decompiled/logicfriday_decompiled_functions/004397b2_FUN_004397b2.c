/* 004397b2 FUN_004397b2 */

undefined4 __thiscall FUN_004397b2(void *this,undefined4 param_1,int param_2)

{
  undefined4 uVar1;
  LRESULT LVar2;
  undefined4 local_10;
  undefined4 local_c;
  LRESULT local_8;
  
  local_10 = 0;
  local_c = 0;
  local_8 = SendMessageA(*(HWND *)((int)this + 4),0x45f,(WPARAM)&local_10,0);
  if (param_2 + -1 < local_8) {
    uVar1 = 0xffffffff;
  }
  else {
    *(undefined4 *)((int)this + 0x10) = param_1;
    *(undefined4 *)((int)this + 0xc) = 0xffffffff;
    *(undefined4 *)((int)this + 8) = 0;
    LVar2 = SendMessageA(*(HWND *)((int)this + 4),1099,0,(int)this + 8);
    *(LRESULT *)((int)this + 0x58) = LVar2;
    if (*(uint *)((int)this + 0x58) < 2) {
      FUN_00439a35((int)this);
    }
    uVar1 = *(undefined4 *)((int)this + 0x58);
  }
  return uVar1;
}
