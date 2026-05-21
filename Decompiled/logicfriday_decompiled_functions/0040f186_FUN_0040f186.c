/* 0040f186 FUN_0040f186 */

LRESULT __thiscall FUN_0040f186(void *this,undefined4 *param_1)

{
  LRESULT LVar1;
  LRESULT LVar2;
  WPARAM local_c;
  
  LVar1 = SendMessageA(*(HWND *)((int)this + 4),0x1032,0,0);
  if (LVar1 == 0) {
    LVar1 = 0;
  }
  else {
    for (local_c = 0; (int)local_c < *(int *)((int)this + 0x50); local_c = local_c + 1) {
      LVar2 = SendMessageA(*(HWND *)((int)this + 4),0x102c,local_c,2);
      if (LVar2 == 2) {
        *(WPARAM *)((int)this + 0x2c) = local_c;
        *(undefined4 *)((int)this + 0x28) = 4;
        SendMessageA(*(HWND *)((int)this + 4),0x1005,0,(int)this + 0x28);
        *param_1 = *(undefined4 *)((int)this + 0x48);
        *(WPARAM *)((int)this + 0x54) = local_c;
        return LVar1;
      }
    }
  }
  return LVar1;
}
