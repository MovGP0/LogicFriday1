/* 0044354d FUN_0044354d */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __cdecl FUN_0044354d(undefined4 *param_1,undefined1 *param_2,size_t param_3,int param_4)

{
  char *pcVar1;
  int iVar2;
  char *pcVar3;
  uint unaff_retaddr;
  uint local_30 [6];
  int local_18;
  int local_14;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  FUN_00447d4c(*param_1,param_1[1],&local_18,local_30);
  iVar2 = local_14 + -1;
  FUN_00447c1b(param_2 + (local_18 == 0x2d),param_3,(int)&local_18);
  local_14 = local_14 + -1;
  if ((local_14 < -4) || ((int)param_3 <= local_14)) {
    __cftoe2(param_3,param_4,'\x01');
  }
  else {
    pcVar1 = param_2 + (local_18 == 0x2d);
    if (iVar2 < local_14) {
      do {
        pcVar3 = pcVar1;
        pcVar1 = pcVar3 + 1;
      } while (*pcVar3 != '\0');
      pcVar3[-1] = '\0';
    }
    FUN_00443449(param_2,param_3,'\x01');
  }
  return;
}
