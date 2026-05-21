/* 0043a7eb FUN_0043a7eb */

undefined4 __thiscall
FUN_0043a7eb(void *this,HWND param_1,undefined4 param_2,uint param_3,undefined4 param_4)

{
  int local_c;
  int local_8;
  
  if (*(int *)((int)this + 0x44) != 0) {
    local_c = (int)(short)param_4;
    local_8 = (int)(short)((uint)param_4 >> 0x10);
    FUN_0043a9ff(this,&local_c);
    FUN_0043a5a9(this,local_c,local_8,param_3);
    *(undefined4 *)((int)this + 0x44) = 0;
    if (*(int *)((int)this + 0x6c) == 0) {
      *(int *)((int)this + 0x28) = local_8;
    }
    else if (*(int *)((int)this + 0x6c) == 1) {
      *(int *)((int)this + 0x2c) = local_c;
    }
    else {
      *(undefined4 *)((int)this + 0x6c) = 2;
      *(int *)((int)this + 0x30) = local_8;
    }
    ReleaseCapture();
    FUN_00439d71(this,0,0);
    if (*(int *)((int)this + 0x70) != 0) {
      *(undefined4 *)(*(int *)((int)this + 0x70) + 0x167c) = *(undefined4 *)((int)this + 0x28);
      *(undefined4 *)(*(int *)((int)this + 0x70) + 0x1680) = *(undefined4 *)((int)this + 0x2c);
      *(undefined4 *)(*(int *)((int)this + 0x70) + 0x1684) = *(undefined4 *)((int)this + 0x30);
    }
    InvalidateRect(param_1,(RECT *)0x0,1);
    UpdateWindow(param_1);
  }
  return 0;
}
