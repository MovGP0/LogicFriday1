/* 0043ac51 FUN_0043ac51 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0043ac51(void *this,int param_1,int param_2)

{
  int iVar1;
  int iVar2;
  undefined4 uVar3;
  void *pvVar4;
  uint unaff_retaddr;
  char local_28 [32];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  if ((*(int *)((int)this + 0x28) < 2) ||
     (((*(int *)(*(int *)((int)this + 0x2c) + 0xc + (*(int *)((int)this + 0x28) + -2) * 0x14) != 0
       || (param_1 !=
           *(int *)((*(int *)((int)this + 0x28) + -2) * 0x14 + *(int *)((int)this + 0x2c)))) &&
      ((*(int *)(*(int *)((int)this + 0x2c) + 0xc + (*(int *)((int)this + 0x28) + -2) * 0x14) != 1
       || (param_2 !=
           *(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -2) * 0x14)))))))
  {
    *(int *)((int)this + 0x28) = *(int *)((int)this + 0x28) + 1;
    pvVar4 = _realloc(*(void **)((int)this + 0x2c),*(int *)((int)this + 0x28) * 0x14);
    *(void **)((int)this + 0x2c) = pvVar4;
    if (*(int *)((int)this + 0x2c) == 0) {
      uVar3 = 0;
    }
    else {
      iVar2 = (*(int *)((int)this + 0x28) + -1) * 0x14;
      iVar1 = *(int *)((int)this + 0x2c);
      *(int *)(iVar1 + iVar2) = param_1;
      *(int *)(iVar1 + 4 + iVar2) = param_2;
      *(undefined4 *)(*(int *)((int)this + 0x2c) + 8 + (*(int *)((int)this + 0x28) + -1) * 0x14) = 0
      ;
      *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10 + (*(int *)((int)this + 0x28) + -1) * 0x14)
           = *(undefined4 *)((int)this + 0x48);
      *(int *)((int)this + 0x48) = *(int *)((int)this + 0x48) + 1;
      if (1 < *(int *)((int)this + 0x28)) {
        if (*(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -1) * 0x14) ==
            *(int *)(*(int *)((int)this + 0x2c) + 4 + (*(int *)((int)this + 0x28) + -2) * 0x14)) {
          *(undefined4 *)
           (*(int *)((int)this + 0x2c) + 0xc + (*(int *)((int)this + 0x28) + -2) * 0x14) = 1;
        }
        else {
          *(undefined4 *)
           (*(int *)((int)this + 0x2c) + 0xc + (*(int *)((int)this + 0x28) + -2) * 0x14) = 0;
        }
      }
      if (DAT_00452ef4 != 0) {
        FUN_0043ed39(local_28,(byte *)"iNodeCnt = %d");
        FUN_0040bdc3((LPARAM)local_28);
      }
      uVar3 = 1;
    }
  }
  else {
    iVar2 = (*(int *)((int)this + 0x28) + -1) * 0x14;
    iVar1 = *(int *)((int)this + 0x2c);
    *(int *)(iVar1 + iVar2) = param_1;
    *(int *)(iVar1 + 4 + iVar2) = param_2;
    uVar3 = 1;
  }
  return uVar3;
}
