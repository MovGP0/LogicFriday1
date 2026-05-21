/* 0041c423 FUN_0041c423 */

uint FUN_0041c423(LPSTR param_1,int *param_2,int *param_3,uint param_4)

{
  char *_Str;
  uint uVar1;
  size_t sVar2;
  uint uVar3;
  size_t local_10;
  uint local_c;
  
  _Str = (char *)*param_2;
  sVar2 = _strlen(_Str);
  if (sVar2 < param_4) {
    local_10 = _strlen(_Str);
  }
  else {
    local_10 = param_4;
  }
  for (local_c = 0; (int)local_c < (int)local_10; local_c = local_c + 1) {
    if (_Str[local_c] == '\n') {
      lstrcpynA(param_1,_Str,local_c + 1);
      *param_2 = local_c + 1 + *param_2;
      *param_3 = *param_3 - (local_c + 1);
      return local_c;
    }
  }
  sVar2 = _strlen(_Str);
  uVar1 = param_4;
  if (param_4 < sVar2) {
    do {
      uVar3 = uVar1;
      local_c = uVar3 - 1;
      if ((int)local_c < 0) {
        lstrcpynA(param_1,_Str,param_4 + 1);
        *param_2 = *param_2 + param_4;
        *param_3 = *param_3 - param_4;
        return param_4;
      }
    } while ((_Str[local_c] != ' ') && (uVar1 = local_c, _Str[local_c] != '+'));
    lstrcpynA(param_1,_Str,uVar3 + 1);
    *param_2 = uVar3 + *param_2;
    *param_3 = *param_3 - uVar3;
  }
  else {
    lstrcpynA(param_1,_Str,param_4 + 1);
    *param_3 = 0;
    uVar3 = param_4;
  }
  return uVar3;
}
