/* 00411caa FUN_00411caa */

int FUN_00411caa(uint *param_1,char *param_2,int param_3)

{
  uint uVar1;
  uint uVar2;
  char cVar3;
  char cVar4;
  char *pcVar5;
  int iVar6;
  uint local_24;
  uint local_20;
  uint local_18;
  int local_14;
  uint local_8;
  
  local_14 = 0;
  uVar1 = param_1[0x31];
  pcVar5 = _strchr(param_2,0x58);
  local_8 = uVar1;
  if (pcVar5 == (char *)0x0) {
    for (; -1 < (int)local_8; local_8 = local_8 - 1) {
      if (param_2[uVar1 - local_8] == '1') {
        local_14 = local_14 + (1 << ((char)local_8 - 1U & 0x1f));
      }
    }
    iVar6 = FUN_00411dfc((int)param_1,local_14,param_3);
    if (iVar6 != 0) {
      return iVar6;
    }
  }
  else {
    local_20 = 0;
    local_18 = 0;
    uVar2 = *param_1;
    for (local_8 = 0; (int)local_8 < (int)uVar1; local_8 = local_8 + 1) {
      cVar3 = (char)uVar1;
      cVar4 = (char)local_8;
      if (param_2[local_8] == '1') {
        local_20 = local_20 | 1 << ((cVar3 - cVar4) - 1U & 0x1f);
      }
      else if (param_2[local_8] == 'X') {
        local_20 = local_20 | 1 << ((cVar3 - cVar4) - 1U & 0x1f);
        local_18 = local_18 | 1 << ((cVar3 - cVar4) - 1U & 0x1f);
      }
    }
    for (local_24 = 0; local_24 < uVar2; local_24 = local_24 + 1) {
      if (((local_24 | local_18) == local_20) &&
         (iVar6 = FUN_00411dfc((int)param_1,local_24,param_3), iVar6 != 0)) {
        return iVar6;
      }
    }
  }
  return 0;
}
