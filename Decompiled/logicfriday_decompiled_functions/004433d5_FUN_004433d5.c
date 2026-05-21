/* 004433d5 FUN_004433d5 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __cdecl FUN_004433d5(undefined4 *param_1,int param_2,int param_3,int param_4)

{
  uint unaff_retaddr;
  uint local_30 [6];
  int local_18 [4];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  FUN_00447d4c(*param_1,param_1[1],local_18,local_30);
  FUN_00447c1b((char *)((uint)(0 < param_3) + param_2 + (uint)(local_18[0] == 0x2d)),param_3 + 1,
               (int)local_18);
  __cftoe2(param_3,param_4,'\0');
  return param_2;
}
