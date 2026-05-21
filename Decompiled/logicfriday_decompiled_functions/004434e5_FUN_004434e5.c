/* 004434e5 FUN_004434e5 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined1 * __cdecl FUN_004434e5(undefined4 *param_1,undefined1 *param_2,size_t param_3)

{
  uint unaff_retaddr;
  uint local_30 [6];
  int local_18;
  int local_14;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  FUN_00447d4c(*param_1,param_1[1],&local_18,local_30);
  FUN_00447c1b(param_2 + (local_18 == 0x2d),local_14 + param_3,(int)&local_18);
  FUN_00443449(param_2,param_3,'\0');
  return param_2;
}
