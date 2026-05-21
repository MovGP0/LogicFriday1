/* 0041e12f FUN_0041e12f */

int __thiscall
FUN_0041e12f(void *this,undefined4 *param_1,int *param_2,undefined4 param_3,uint *param_4)

{
  int iVar1;
  uint *puVar2;
  int local_8;
  
  local_8 = 0;
  if (DAT_00452e84 != 0) {
    local_8 = FUN_00420265(this);
  }
  if (local_8 == 0) {
    *(undefined4 *)((int)this + 0x240) = 1;
    *(undefined4 *)((int)this + 0x264) = 7;
    *param_1 = this;
    *param_2 = (int)this + 0x1f0;
    puVar2 = (uint *)((int)this + 0x16d8);
    for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
      *param_4 = *puVar2;
      puVar2 = puVar2 + 1;
      param_4 = param_4 + 1;
    }
    local_8 = 0;
  }
  return local_8;
}
