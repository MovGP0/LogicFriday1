/* 00447b95 FUN_00447b95 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __cdecl FUN_00447b95(_CRT_DOUBLE *param_1,byte *param_2)

{
  uint unaff_retaddr;
  _LDBL12 local_18;
  int local_c;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  FUN_004487e3((undefined2 *)&local_18,&local_c,param_2,0,0,0,0);
  FID_conflict___ld12tod(&local_18,param_1);
  return;
}
