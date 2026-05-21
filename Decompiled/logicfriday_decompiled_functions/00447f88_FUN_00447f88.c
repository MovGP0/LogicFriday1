/* 00447f88 FUN_00447f88 */

void FUN_00447f88(void)

{
  SetUnhandledExceptionFilter(DAT_0046c9b4);
  return;
}



/* 00447f95 FID_conflict:_ValidateRead */

/* Library Function - Multiple Matches With Different Base Names
    int __cdecl _ValidateRead(void const *,unsigned int)
    int __cdecl _ValidateWrite(void *,unsigned int)
   
   Library: Visual Studio 2003 Release */

bool __cdecl FID_conflict__ValidateRead(void *param_1,UINT_PTR param_2)

{
  BOOL BVar1;
  
  BVar1 = IsBadReadPtr(param_1,param_2);
  return BVar1 == 0;
}



/* 00447fb1 FID_conflict:_ValidateRead */

/* Library Function - Multiple Matches With Different Base Names
    int __cdecl _ValidateRead(void const *,unsigned int)
    int __cdecl _ValidateWrite(void *,unsigned int)
   
   Library: Visual Studio 2003 Release */

bool __cdecl FID_conflict__ValidateRead(LPVOID param_1,UINT_PTR param_2)

{
  BOOL BVar1;
  
  BVar1 = IsBadWritePtr(param_1,param_2);
  return BVar1 == 0;
}
