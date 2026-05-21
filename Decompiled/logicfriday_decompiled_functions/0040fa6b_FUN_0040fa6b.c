/* 0040fa6b FUN_0040fa6b */

undefined4 __thiscall FUN_0040fa6b(void *this,int param_1)

{
  LRESULT LVar1;
  WPARAM local_c;
  WPARAM local_8;
  
  *(undefined4 *)((int)this + 0x28) = 4;
  local_8 = 0;
  while( true ) {
    if (*(int *)((int)this + 0x50) <= (int)local_8) {
      return 0;
    }
    *(WPARAM *)((int)this + 0x2c) = local_8;
    SendMessageA(*(HWND *)((int)this + 4),0x1005,0,(int)this + 0x28);
    if (*(int *)((int)this + 0x48) == param_1) break;
    local_8 = local_8 + 1;
  }
  SendMessageA(*(HWND *)((int)this + 4),0x1008,local_8,0);
  *(int *)((int)this + 0x50) = *(int *)((int)this + 0x50) + -1;
  for (local_c = 0; (int)local_c < *(int *)((int)this + 0x50); local_c = local_c + 1) {
    LVar1 = SendMessageA(*(HWND *)((int)this + 4),0x102c,local_c,2);
    if (LVar1 == 2) {
      *(WPARAM *)((int)this + 0x54) = local_c;
    }
    else {
      *(undefined4 *)((int)this + 0x54) = 0;
    }
  }
  return 1;
}
