/* 00436863 FUN_00436863 */

undefined4 __thiscall FUN_00436863(void *this,char *param_1)

{
  size_t sVar1;
  undefined4 uVar2;
  int iVar3;
  int local_c;
  
  sVar1 = _strlen(param_1);
  if ((sVar1 == 0) || (8 < sVar1)) {
    uVar2 = 1;
  }
  else {
    for (local_c = 0; local_c < *(int *)((int)this + 0xc4); local_c = local_c + 1) {
      iVar3 = _strcmp((char *)((int)this + local_c * 9 + 0x160),param_1);
      if (iVar3 == 0) {
        return 2;
      }
    }
    for (local_c = 0; local_c < *(int *)((int)this + 200); local_c = local_c + 1) {
      iVar3 = _strcmp((char *)((int)this + local_c * 9 + 0xd0),param_1);
      if (iVar3 == 0) {
        return 2;
      }
    }
    uVar2 = FUN_0040daf0(param_1);
  }
  return uVar2;
}
