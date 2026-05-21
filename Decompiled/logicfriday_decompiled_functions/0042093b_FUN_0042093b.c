/* 0042093b FUN_0042093b */

int __thiscall FUN_0042093b(void *this,int param_1,char *param_2)

{
  int iVar1;
  
  if (*(int *)((int)this + 0x264) == 7) {
    iVar1 = FUN_004209b2(this,param_1,param_2);
  }
  else {
    if (*(int *)((int)this + 0x264) != 8) {
      MessageBoxA(*(HWND *)((int)this + 0x16f0),"Bad equation format.","Error",0);
      return 0;
    }
    iVar1 = FUN_00421093(this);
  }
  if (iVar1 == 0) {
    return 0;
  }
  return iVar1;
}
