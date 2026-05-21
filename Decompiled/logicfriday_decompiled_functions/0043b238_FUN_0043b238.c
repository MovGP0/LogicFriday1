/* 0043b238 FUN_0043b238 */

bool __thiscall FUN_0043b238(void *this,int param_1,int param_2,int param_3,int param_4)

{
  bool bVar1;
  int iVar2;
  int iVar3;
  
  bVar1 = false;
  if (1 < *(int *)((int)this + 0x28)) {
    if (*(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -1) * 0x14) ==
        *(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -2) * 0x14)) {
      if (*(int *)((*(int *)((int)this + 0x28) + -1) * 0x14 + *(int *)((int)this + 0x2c)) <
          *(int *)((*(int *)((int)this + 0x28) + -2) * 0x14 + *(int *)((int)this + 0x2c))) {
        bVar1 = *(int *)((*(int *)((int)this + 0x28) + -1) * 0x14 + *(int *)((int)this + 0x2c)) <
                param_1;
      }
      else if (param_1 < *(int *)((*(int *)((int)this + 0x28) + -1) * 0x14 +
                                 *(int *)((int)this + 0x2c))) {
        bVar1 = true;
      }
    }
    else if (*(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -1) * 0x14) <
             *(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -2) * 0x14)) {
      bVar1 = *(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -1) * 0x14) <
              param_2;
    }
    else if (param_2 < *(int *)(*(int *)((int)this + 0x2c) + 4 +
                               (*(int *)((int)this + 0x28) + -1) * 0x14)) {
      bVar1 = true;
    }
  }
  if (bVar1) {
    iVar2 = (*(int *)((int)this + 0x28) + -1) * 0x14;
    iVar3 = *(int *)((int)this + 0x2c);
    *(int *)(iVar3 + iVar2) = param_1;
    *(int *)(iVar3 + 4 + iVar2) = param_2;
  }
  else {
    iVar3 = FUN_0043ac51(this,param_1,param_2);
    if (iVar3 == 0) {
      return false;
    }
  }
  iVar3 = FUN_0043ac51(this,param_3,param_4);
  return iVar3 != 0;
}
