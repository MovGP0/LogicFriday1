/* 0043ae30 FUN_0043ae30 */

undefined4 __thiscall
FUN_0043ae30(void *this,int param_1,int param_2,undefined4 param_3,undefined4 param_4,int param_5)

{
  int iVar1;
  void *pvVar2;
  undefined4 uVar3;
  int iVar4;
  POINT local_44 [2];
  int local_34;
  int local_18;
  int local_14;
  int local_c;
  int local_8;
  
  local_c = 0;
  local_44[0].x = param_1;
  local_44[0].y = param_2;
  FUN_0043bad3(this,local_44);
  *(int *)((int)this + 0x30) = *(int *)((int)this + 0x30) + 1;
  pvVar2 = _realloc(*(void **)((int)this + 0x34),*(int *)((int)this + 0x30) * 0x14);
  *(void **)((int)this + 0x34) = pvVar2;
  if (*(int *)((int)this + 0x34) == 0) {
    uVar3 = 0;
  }
  else {
    iVar4 = (*(int *)((int)this + 0x30) + -1) * 0x14;
    iVar1 = *(int *)((int)this + 0x34);
    *(int *)(iVar1 + iVar4) = param_1;
    *(int *)(iVar1 + 4 + iVar4) = param_2;
    *(undefined4 *)(*(int *)((int)this + 0x34) + 0x10 + (*(int *)((int)this + 0x30) + -1) * 0x14) =
         param_4;
    *(undefined4 *)(*(int *)((int)this + 0x34) + 0xc + (*(int *)((int)this + 0x30) + -1) * 0x14) =
         param_3;
    for (local_8 = 0; local_8 < *(int *)((int)this + 0x28); local_8 = local_8 + 1) {
      if ((*(int *)(local_8 * 0x14 + *(int *)((int)this + 0x2c)) == param_1) &&
         (*(int *)(*(int *)((int)this + 0x2c) + 4 + local_8 * 0x14) == param_2)) {
        local_c = 1;
        break;
      }
    }
    if ((local_c == 0) || (local_34 != param_5)) {
      *(undefined4 *)(*(int *)((int)this + 0x34) + 8 + (*(int *)((int)this + 0x30) + -1) * 0x14) =
           *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10 + local_18 * 0x14);
    }
    else {
      *(undefined4 *)(*(int *)((int)this + 0x34) + 8 + (*(int *)((int)this + 0x30) + -1) * 0x14) =
           *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10 + local_14 * 0x14);
    }
    uVar3 = 1;
  }
  return uVar3;
}
