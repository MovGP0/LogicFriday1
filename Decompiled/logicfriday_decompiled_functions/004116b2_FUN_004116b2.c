/* 004116b2 FUN_004116b2 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_004116b2(uint *param_1,uint *param_2,uint *param_3)

{
  undefined4 uVar1;
  size_t sVar2;
  uint unaff_retaddr;
  uint *local_418;
  uint local_414 [257];
  uint local_10;
  char local_c [4];
  char *local_8;
  
  local_10 = DAT_00451a00 ^ unaff_retaddr;
  local_8 = (char *)0x0;
  local_418 = (uint *)0x0;
  local_c[0] = ',';
  local_c[1] = '\t';
  local_c[2] = 0;
  FUN_0043ebd0(param_2,(uint *)&DAT_0044ad26);
  FUN_0043ebd0(param_3,(uint *)&DAT_0044ad26);
  FUN_0043ebd0(local_414,param_1);
  local_418 = local_414;
  local_8 = FUN_00415979(&local_418,local_c);
  if (local_8 == (char *)0x0) {
    uVar1 = 1;
  }
  else {
    while (sVar2 = _strlen(local_8), sVar2 != 0) {
      if (((*local_8 != '0') && (*local_8 != '1')) && ((*local_8 != 'X' && (*local_8 != 'x')))) {
        return 1;
      }
      if (*local_8 == 'x') {
        *local_8 = 'X';
      }
      _strncat((char *)param_2,local_8,1);
      local_8 = FUN_00415979(&local_418,local_c);
    }
    while (sVar2 = _strlen(local_8), sVar2 == 0) {
      local_8 = FUN_00415979(&local_418,local_c);
    }
    while (local_8 != (char *)0x0) {
      if ((((*local_8 != '0') && (*local_8 != '1')) && (*local_8 != 'X')) && (*local_8 != 'x')) {
        return 1;
      }
      if (*local_8 == 'x') {
        *local_8 = 'X';
      }
      _strncat((char *)param_3,local_8,1);
      local_8 = FUN_00415979(&local_418,local_c);
    }
    uVar1 = 0;
  }
  return uVar1;
}
