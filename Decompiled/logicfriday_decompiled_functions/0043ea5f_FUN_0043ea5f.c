/* 0043ea5f FUN_0043ea5f */

void FUN_0043ea5f(void)

{
  _ptiddata p_Var1;
  
  if (PTR_FUN_00451a20 != (undefined *)0x0) {
    (*(code *)PTR_FUN_00451a20)();
  }
  p_Var1 = __getptd();
  if (p_Var1 == (_ptiddata)0x0) {
    __amsg_exit(0x10);
  }
  if ((HANDLE)p_Var1->_thandle != (HANDLE)0xffffffff) {
    CloseHandle((HANDLE)p_Var1->_thandle);
  }
  FUN_00442c1a(p_Var1);
                    /* WARNING: Subroutine does not return */
  ExitThread(0);
}
