/* 00441a2d FUN_00441a2d */

ulong * FUN_00441a2d(void)

{
  _ptiddata p_Var1;
  
  p_Var1 = __getptd();
  return &p_Var1->_tdoserrno;
}
