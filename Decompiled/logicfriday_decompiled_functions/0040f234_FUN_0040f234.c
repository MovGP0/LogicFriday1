/* 0040f234 FUN_0040f234 */

undefined4 __thiscall FUN_0040f234(void *this,int *param_1,undefined4 *param_2)

{
  LRESULT LVar1;
  undefined4 uVar2;
  undefined4 local_c;
  
  *param_2 = 0;
  *param_1 = 0;
  LVar1 = SendMessageA(*(HWND *)((int)this + 4),0x1032,0,0);
  if (LVar1 == 2) {
    for (local_c = 0; (int)local_c < *(int *)((int)this + 0x50); local_c = local_c + 1) {
      LVar1 = SendMessageA(*(HWND *)((int)this + 4),0x102c,local_c,2);
      if (LVar1 == 2) {
        *(WPARAM *)((int)this + 0x2c) = local_c;
        *(undefined4 *)((int)this + 0x28) = 4;
        SendMessageA(*(HWND *)((int)this + 4),0x1005,0,(int)this + 0x28);
        if (*param_1 != 0) {
          *param_2 = *(undefined4 *)((int)this + 0x48);
          break;
        }
        *param_1 = *(int *)((int)this + 0x48);
      }
    }
    uVar2 = 2;
  }
  else {
    uVar2 = 0;
  }
  return uVar2;
}
